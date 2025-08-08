#!/usr/bin/env python3
import json
import sys
from typing import Any, Iterator, Optional, Union, List


def main():
    send_id = send(method="ready")
    recv(expect_id=send_id)

    for request in listen():
        request_id = request.get("id")

        method = request.get("method")
        if not method:
            fatal_error(f"expected method, got {request!r}", in_reply_to=request_id)
        elif method == "shutdown":
            break
        elif method == "invoke":
            params = request.get("params", {})
            selector = params.get("selector", "")
            calldata = decode(params.get("calldata", []))
            try:
                result = route_invoke(selector, calldata)
                send(id=request_id, result=encode(result))
            except Exception as e:
                send(id=request_id, error=e)
        else:
            send(id=request_id, error=f"unknown method {method!r}")


def route_invoke(selector, calldata: List[int]) -> List[int]:
    if selector == "sqrt":
        return [int(calldata[0] ** 0.5)]
    elif selector == "panic":
        raise ValueError("oops")
    else:
        raise ValueError(f"unknown selector {selector!r}")


_next_send_id = 0


def send(
    *,
    id: Optional[int] = None,
    method: Optional[str] = None,
    params: Optional[dict] = None,
    result: Optional[Any] = None,
    error: Optional[Union[str, Exception]] = None,
) -> int:
    global _next_send_id
    if id is None:
        id = _next_send_id
        _next_send_id += 1

    response: dict[str, Any] = {"jsonrpc": "2.0", "id": id}

    if method is not None:
        response["method"] = method

    if params is not None:
        response["params"] = params

    if result is not None:
        response["result"] = result

    if error is not None:
        response["error"] = {"code": 0, "message": str(error)}

    print(json.dumps(response), flush=True)

    return id


def recv(*, expect_id: Optional[int] = None) -> dict[str, Any]:
    line = input()
    message = json.loads(line)

    if not isinstance(message, dict):
        fatal_error(f"expected JSON object, got {type(message).__name__}: {message!r}")

    if message.get("jsonrpc") != "2.0":
        fatal_error(f"expected JSON-RPC 2.0, got {message.get('jsonrpc')!r}")

    if expect_id is not None and message.get("id") != expect_id:
        fatal_error(
            f"expected message with ID {expect_id!r}, got {message.get('id')!r}",
            in_reply_to=message.get("id"),
        )

    return message


def listen() -> Iterator[dict[str, Any]]:
    try:
        while True:
            yield recv()
    except EOFError:
        pass


def fatal_error(err: str, /, in_reply_to: Optional[int] = None):
    print(err, file=sys.stderr)
    if in_reply_to is not None:
        send(id=in_reply_to, error=err)
    sys.exit(1)


def encode(result: List[int]) -> List[str]:
    return [hex(felt) for felt in result]


def decode(calldata: List[Union[int, str]]) -> List[int]:
    return [int(felt, 0) if isinstance(felt, str) else felt for felt in calldata]


if __name__ == "__main__":
    main()
