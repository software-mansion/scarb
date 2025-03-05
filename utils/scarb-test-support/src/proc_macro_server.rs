use crate::command::Scarb;
use scarb_proc_macro_server_types::jsonrpc::RequestId;
use scarb_proc_macro_server_types::jsonrpc::ResponseError;
use scarb_proc_macro_server_types::jsonrpc::RpcRequest;
use scarb_proc_macro_server_types::jsonrpc::RpcResponse;
use scarb_proc_macro_server_types::methods::defined_macros::DefinedMacros;
use scarb_proc_macro_server_types::methods::defined_macros::DefinedMacrosParams;
use scarb_proc_macro_server_types::methods::defined_macros::PackageDefinedMacrosInfo;
use scarb_proc_macro_server_types::methods::Method;
use std::collections::HashMap;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Lines;
use std::io::Write;
use std::marker::PhantomData;
use std::path::Path;
use std::process::Child;
use std::process::ChildStdin;
use std::process::ChildStdout;
use std::process::Stdio;

pub const SIMPLE_MACROS_V1: &str = r#"
use cairo_lang_macro::{
    ProcMacroResult,
    TokenStream,
    attribute_macro,
    inline_macro,
    derive_macro,
    executable_attribute
};

executable_attribute!("some_executable");

#[attribute_macro]
pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
    ProcMacroResult::new(token_stream)
}

#[inline_macro]
pub fn inline_some(token_stream: TokenStream) -> ProcMacroResult {
    ProcMacroResult::new(token_stream)
}

#[derive_macro]
fn some_derive(_token_stream: TokenStream)-> ProcMacroResult {
    ProcMacroResult::new(TokenStream::new("impl SomeImpl of SomeTrait {}".to_string()))
}
"#;

pub const SIMPLE_MACROS_V2: &str = r#"
use cairo_lang_macro_v2::{
    ProcMacroResult,
    TokenStream, TokenTree, Token, TextSpan,
    attribute_macro,
    inline_macro,
    derive_macro,
    executable_attribute
};

executable_attribute!("some_executable");

#[attribute_macro]
pub fn some(_attr: TokenStream, token_stream: TokenStream) -> ProcMacroResult {
    ProcMacroResult::new(token_stream)
}

#[inline_macro]
pub fn inline_some(token_stream: TokenStream) -> ProcMacroResult {
    ProcMacroResult::new(token_stream)
}

#[derive_macro]
fn some_derive(_token_stream: TokenStream)-> ProcMacroResult {
    let content = "impl SomeImpl of SomeTrait {}".to_string();
    let span = TextSpan { start: 0, end: content.len() as u32 };
    ProcMacroResult::new(
        TokenStream::new(vec![
            TokenTree::Ident(
                Token::new(content, span)
            )
        ])
    )
}
"#;

pub struct PendingRequest<M: Method> {
    id: RequestId,
    _method: PhantomData<M>,
}

impl<M: Method> PendingRequest<M> {
    fn new(id: RequestId) -> Self {
        Self {
            id,
            _method: Default::default(),
        }
    }
}

pub struct ProcMacroClient {
    requester: ChildStdin,
    responder: Lines<BufReader<ChildStdout>>,
    server_process: Child,
    id_counter: RequestId,
    responses: HashMap<RequestId, RpcResponse>,
}

/// A helper structure containing macros available for a package
/// identified by the `package_id` which is a serialized `PackageId`.
pub struct DefinedMacrosInfo {
    /// An ID of the package recognized by PMS.
    pub package_id: String,
    /// A proper part of the response, related to the main component of the main CU.
    pub defined_macros: PackageDefinedMacrosInfo,
}

impl ProcMacroClient {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let mut server_process = Scarb::new()
            .std()
            .arg("--quiet")
            .arg("proc-macro-server")
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .stderr(Stdio::inherit())
            .current_dir(path)
            .spawn()
            .unwrap();

        let requester = server_process.stdin.take().unwrap();
        let responder = BufReader::new(server_process.stdout.take().unwrap()).lines();

        Self {
            requester,
            responder,
            server_process,
            id_counter: Default::default(),
            responses: Default::default(),
        }
    }
    pub fn new_without_cargo<P: AsRef<Path>>(path: P) -> Self {
        let mut server_process = Scarb::new()
            .std()
            .arg("--quiet")
            .arg("proc-macro-server")
            .env("CARGO", "/bin/false")
            .env("RUSTC", "/bin/false")
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .stderr(Stdio::inherit())
            .current_dir(path)
            .spawn()
            .unwrap();

        let requester = server_process.stdin.take().unwrap();
        let responder = BufReader::new(server_process.stdout.take().unwrap()).lines();

        Self {
            requester,
            responder,
            server_process,
            id_counter: Default::default(),
            responses: Default::default(),
        }
    }

    pub fn request<M: Method>(&mut self, params: M::Params) -> PendingRequest<M> {
        let id = self.id_counter;
        self.id_counter += 1;

        let mut request = serde_json::to_vec(&RpcRequest {
            id,
            method: M::METHOD.to_string(),
            value: serde_json::to_value(params).unwrap(),
        })
        .unwrap();
        request.push(b'\n');

        self.requester.write_all(&request).unwrap();
        self.requester.flush().unwrap();

        PendingRequest::new(id)
    }

    pub fn request_and_wait<M: Method>(
        &mut self,
        params: M::Params,
    ) -> Result<M::Response, ResponseError> {
        let request = self.request(params);

        self.wait_for_response::<M>(request)
    }

    pub fn wait_for_response<M: Method>(
        &mut self,
        request: PendingRequest<M>,
    ) -> Result<M::Response, ResponseError> {
        // If we already read this response, return it.
        if let Some(raw_response) = self.responses.remove(&request.id) {
            return raw_response
                .into_result()
                .map(|value| serde_json::from_value(value).unwrap());
        }

        // Read responses until we get requested one, keeping all others in memory.
        loop {
            let response = self.responder.next().unwrap().unwrap();
            let raw_response: RpcResponse = serde_json::from_str(&response).unwrap();

            if raw_response.id == request.id {
                return raw_response
                    .into_result()
                    .map(|value| serde_json::from_value(value).unwrap());
            } else {
                self.responses.insert(raw_response.id, raw_response);
            }
        }
    }

    /// Returns the information about macros available for the package with name `package_name`.
    /// Used as a helper in PMS tests, where communication requires package IDs assigned by Scarb.
    pub fn defined_macros_for_package(&mut self, package_name: &str) -> DefinedMacrosInfo {
        let response = self
            .request_and_wait::<DefinedMacros>(DefinedMacrosParams {})
            .unwrap();

        let mut response = response.macros_by_package_id;

        // Usually, we can't discover the ID of the mock package used in test, so we extract it from the PMS response.
        let package_id = response
            .keys()
            .find(|cu_id| cu_id.starts_with(package_name))
            .expect("Response from Proc Macro Server should contain the main compilation unit.")
            .to_owned();

        let defined_macros = response.remove(&package_id).expect(
            "Response from Proc Macro Server should contain the main compilation unit component.",
        );

        DefinedMacrosInfo {
            package_id,
            defined_macros,
        }
    }
}

impl Drop for ProcMacroClient {
    fn drop(&mut self) {
        self.server_process.kill().unwrap();
        self.server_process.wait().unwrap();
    }
}
