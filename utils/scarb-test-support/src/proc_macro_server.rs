use crate::command::Scarb;
use scarb_proc_macro_server_types::jsonrpc::RequestId;
use scarb_proc_macro_server_types::jsonrpc::ResponseError;
use scarb_proc_macro_server_types::jsonrpc::RpcRequest;
use scarb_proc_macro_server_types::jsonrpc::RpcResponse;
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

pub const SIMPLE_MACROS: &str = r#"
use cairo_lang_macro::{
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
    let span = TextSpan { start: 0, end: content.len() };
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
}

impl Drop for ProcMacroClient {
    fn drop(&mut self) {
        self.server_process.kill().unwrap();
        self.server_process.wait().unwrap();
    }
}
