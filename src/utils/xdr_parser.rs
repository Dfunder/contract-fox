use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use stellar_xdr::base64::{DecodeError, XdrBase64};
use stellar_xdr::types::{
    Envelope, Operation, OperationBody, TransactionEnvelope, TransactionV1Envelope, HostFunction,
    ScVal, ScAddress, ScSymbol, Hash, InvokeContractArgs, InvokeHostFunctionOp,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SorobanInvocation {
    pub contract_id: String,
    pub function_name: String,
    pub function_args: Vec<String>,
}

#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum ParseError {
    #[error("invalid base64: {0}")]
    InvalidBase64(String),

    #[error("failed to decode xdr: {0}")]
    DecodeXdr(String),

    #[error("transaction is not a Soroban invoke")]
    NotSorobanInvoke,

    #[error("unsupported invoke structure: {0}")]
    UnsupportedInvoke(String),
}

fn contract_id_to_hex(hash: &Hash) -> String {
    // Hash is 32 bytes. Convert to Stellar strkey contract id format not provided here.
    // Keep deterministic hex representation.
    hash.0.iter().map(|b| format!("{:02x}", b)).collect()
}

fn scval_to_string(v: &ScVal) -> String {
    // Minimal, stable formatting for unit tests and debugging.
    // For unknown variants, fall back to a debug string.
    match v {
        ScVal::U64(n) => n.to_string(),
        ScVal::I128Parts { hi, lo } => format!("i128({hi},{lo})"),
        ScVal::Str(s) => s.0.clone(),
        ScVal::Address(addr) => match addr {
            ScAddress::Contract(c) => format!("contract({})", contract_id_to_hex(c)),
            _ => format!("addr({:?})", addr),
        },
        _ => format!("{:?}", v),
    }
}

/// Parse a base64-encoded Soroban transaction envelope XDR and extract the
/// invoked contract, function name, and arguments.
///
/// This focuses on operations of type `invoke_host_function` / `invoke_contract`.
pub fn parse_soroban_invoke(xdr: &str) -> Result<SorobanInvocation, ParseError> {
    let bytes = STANDARD
        .decode(xdr)
        .map_err(|e| ParseError::InvalidBase64(e.to_string()))?;

    let env = TransactionEnvelope::from_xdr(&bytes)
        .map_err(|e| ParseError::DecodeXdr(e.to_string()))?;

    let tx = match env {
        TransactionEnvelope::V1(txv1) => txv1.tx,
        _ => return Err(ParseError::NotSorobanInvoke),
    };

    for op in tx.operations().iter() {
        if let OperationBody::InvokeHostFunction(InvokeHostFunctionOp { host_function, .. }) = &op.body {
            if let HostFunction::InvokeContract(InvokeContractArgs {
                contract_address,
                function_name,
                args,
            }) = host_function
            {
                let contract_id = match contract_address {
                    stellar_xdr::types::ScAddress::Contract(hash) => contract_id_to_hex(hash),
                    _ => return Err(ParseError::UnsupportedInvoke("contract address type".into())),
                };

                let function_name = match function_name {
                    ScSymbol(sym) => sym.clone(),
                    _ => format!("{:?}", function_name),
                };

                let function_args = args.iter().map(scval_to_string).collect();

                return Ok(SorobanInvocation {
                    contract_id,
                    function_name,
                    function_args,
                });
            }
        }
    }

    Err(ParseError::NotSorobanInvoke)
}

#[cfg(test)]
mod tests {
    use super::*;
    use stellar_xdr::types::{
        Transaction, TransactionV1Envelope, TransactionEnvelope, MuxedAccount, PublicKey, Uint256,
        Operation, OperationBody, OperationSource, Preconditions, Memo, SequenceNumber,
        TransactionExt, SorobanTransactionData, SorobanAuthorizationEntry,
        ScVal, Int128Parts, ScAddress, HostFunction, InvokeContractArgs, ScSymbol, Hash,
        InvokeHostFunctionOp, InvokeContractArgs,
    };

    fn make_contract_hash(byte: u8) -> Hash {
        let mut arr = [0u8; 32];
        arr[0] = byte;
        Hash(arr)
    }

    fn make_invoke_envelope(
        contract_byte: u8,
        function_name: &str,
        args: Vec<ScVal>,
    ) -> String {
        let contract_address = ScAddress::Contract(make_contract_hash(contract_byte));
        let invoke_args = InvokeContractArgs {
            contract_address,
            function_name: ScSymbol(function_name.try_into().unwrap()),
            args: args.try_into().unwrap(),
        };

        let host_function = HostFunction::InvokeContract(invoke_args);
        let op_body = OperationBody::InvokeHostFunction(InvokeHostFunctionOp {
            host_function,
            auth: vec![].try_into().unwrap(),
        });

        let op = Operation {
            source_account: None,
            body: op_body,
        };

        let tx = TransactionV1Envelope {
            tx: Transaction {
                source_account: MuxedAccount::Ed25519(Uint256([0u8; 32])),
                fee: 100,
                seq_num: SequenceNumber(1),
                cond: Preconditions::None,
                memo: Memo::None,
                operations: vec![op].try_into().unwrap(),
                ext: TransactionExt::V0,
            },
            signatures: vec![].try_into().unwrap(),
        };

        let env = TransactionEnvelope::V1(tx);
        let bytes = env.to_xdr();
        STANDARD.encode(bytes)
    }

    #[test]
    fn parses_basic_invoke() {
        let xdr = make_invoke_envelope(
            7,
            "donate",
            vec![
                ScVal::U64(42),
                ScVal::Str(stellar_xdr::types::StringM("hello".into())),
                ScVal::U64(9),
            ],
        );

        let parsed = parse_soroban_invoke(&xdr).unwrap();
        assert_eq!(parsed.function_name, "donate");
        assert_eq!(parsed.function_args.len(), 3);
        assert_eq!(parsed.function_args[0], "42");
    }
}

