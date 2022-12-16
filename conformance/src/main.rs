use std::io::{self, Read, Write};

use bytes::{Buf, BufMut};
use prost::Message;

use protobuf::conformance::{
    conformance_request, conformance_response, ConformanceRequest, ConformanceResponse, WireFormat,
};
use protobuf::test_messages::proto2::TestAllTypesProto2;
use protobuf::test_messages::proto3::TestAllTypesProto3;
use tests::{roundtrip, RoundtripResult};

fn main() -> io::Result<()> {
    env_logger::init();
    let mut bytes = Vec::new();

    loop {
        bytes.resize(4, 0);

        if io::stdin().read_exact(&mut bytes).is_err() {
            // No more test cases.
            return Ok(());
        }

        let len = bytes.as_slice().get_u32_le() as usize;

        bytes.resize(len, 0);
        io::stdin().read_exact(&mut bytes)?;

        let result = match ConformanceRequest::decode(&*bytes) {
            Ok(request) => handle_request(request),
            Err(error) => conformance_response::Result::ParseError(format!("{:?}", error)),
        };

        let response = ConformanceResponse {
            result: Some(result),
        };

        let len = response.encoded_len();
        bytes.clear();
        bytes.put_u32_le(len as u32);
        response.encode(&mut bytes)?;
        assert_eq!(len + 4, bytes.len());

        let mut stdout = io::stdout();
        stdout.lock().write_all(&bytes)?;
        stdout.flush()?;
    }
}

fn handle_request(request: ConformanceRequest) -> conformance_response::Result {
    match request.requested_output_format() {
        WireFormat::Unspecified => {
            return conformance_response::Result::ParseError(
                "output format unspecified".to_string(),
            );
        }
        WireFormat::Json => {
            return conformance_response::Result::Skipped(
                "JSON output is not supported".to_string(),
            );
        }
        WireFormat::Jspb => {
            return conformance_response::Result::Skipped(
                "JSPB output is not supported".to_string(),
            );
        }
        WireFormat::TextFormat => {
            return conformance_response::Result::Skipped(
                "TEXT_FORMAT output is not supported".to_string(),
            );
        }
        WireFormat::Protobuf => (),
    };

    let buf = match request.payload {
        None => return conformance_response::Result::ParseError("no payload".to_string()),
        Some(conformance_request::Payload::JsonPayload(_)) => {
            return conformance_response::Result::Skipped(
                "JSON input is not supported".to_string(),
            );
        }
        Some(conformance_request::Payload::JspbPayload(_)) => {
            return conformance_response::Result::Skipped(
                "JSON input is not supported".to_string(),
            );
        }
        Some(conformance_request::Payload::TextPayload(_)) => {
            return conformance_response::Result::Skipped(
                "JSON input is not supported".to_string(),
            );
        }
        Some(conformance_request::Payload::ProtobufPayload(buf)) => buf,
    };

    let roundtrip = match &*request.message_type {
        "protobuf_test_messages.proto2.TestAllTypesProto2" => roundtrip::<TestAllTypesProto2>(&buf),
        "protobuf_test_messages.proto3.TestAllTypesProto3" => roundtrip::<TestAllTypesProto3>(&buf),
        _ => {
            return conformance_response::Result::ParseError(format!(
                "unknown message type: {}",
                request.message_type
            ));
        }
    };

    match roundtrip {
        RoundtripResult::Ok(buf) => conformance_response::Result::ProtobufPayload(buf),
        RoundtripResult::DecodeError(error) => {
            conformance_response::Result::ParseError(error.to_string())
        }
        RoundtripResult::Error(error) => {
            conformance_response::Result::RuntimeError(error.to_string())
        }
    }
}
