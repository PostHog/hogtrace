use crate::constant_pool::{Constant, ConstantPool};

// Include the generated protobuf code
pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/hogtrace.vm.rs"));
}

/// A compiled HogTrace program ready for execution
#[derive(Debug, Clone)]
pub struct Program {
    /// Bytecode format version
    pub version: u32,

    /// Shared constant pool for all probes
    pub constant_pool: ConstantPool,

    /// All probes in the program
    pub probes: Vec<Probe>,

    /// Global sampling rate (0.0 = no sampling, 1.0 = 100% sampling)
    pub sampling: f32,
}

/// A single probe with its specification and bytecode
#[derive(Debug, Clone)]
pub struct Probe {
    /// Unique probe identifier
    pub id: String,

    /// Probe specification (where to install the probe)
    pub spec: ProbeSpec,

    /// Predicate bytecode (empty if no predicate)
    pub predicate: Vec<u8>,

    /// Action body bytecode
    pub body: Vec<u8>,
}

/// Probe specification - defines where the probe is installed
#[derive(Debug, Clone)]
pub enum ProbeSpec {
    /// Function probe (fn:module.function:target)
    Fn { specifier: String, target: FnTarget },
}

/// Function probe target
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FnTarget {
    Entry,
    Exit,
}

impl Program {
    /// Deserialize a Program from protobuf bytes
    pub fn from_proto_bytes(bytes: &[u8]) -> Result<Self, String> {
        use prost::Message;

        let proto_program = proto::Program::decode(bytes)
            .map_err(|e| format!("Failed to decode protobuf: {}", e))?;

        Self::from_proto(proto_program)
    }

    /// Convert from protobuf Program message
    pub fn from_proto(proto: proto::Program) -> Result<Self, String> {
        let constant_pool = Self::convert_constant_pool(proto.constant_pool)?;
        let probes = proto
            .probes
            .into_iter()
            .map(Probe::from_proto)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Program {
            version: proto.version,
            constant_pool,
            probes,
            sampling: proto.sampling,
        })
    }

    /// Serialize this Program to protobuf bytes
    pub fn to_proto_bytes(&self) -> Result<Vec<u8>, String> {
        use prost::Message;

        let proto = self.to_proto()?;
        let mut buf = Vec::new();
        proto
            .encode(&mut buf)
            .map_err(|e| format!("Failed to encode protobuf: {}", e))?;
        Ok(buf)
    }

    /// Convert to protobuf Program message
    pub fn to_proto(&self) -> Result<proto::Program, String> {
        Ok(proto::Program {
            version: self.version,
            constant_pool: Some(Self::convert_constant_pool_to_proto(&self.constant_pool)?),
            probes: self
                .probes
                .iter()
                .map(Probe::to_proto)
                .collect::<Result<Vec<_>, _>>()?,
            sampling: self.sampling,
        })
    }

    /// Convert protobuf ConstantPool to our ConstantPool
    fn convert_constant_pool(
        proto_pool: Option<proto::ConstantPool>,
    ) -> Result<ConstantPool, String> {
        let proto_pool = proto_pool.ok_or("Missing constant pool")?;
        let mut pool = ConstantPool::new();

        for proto_const in proto_pool.constants {
            let constant = Self::convert_constant(proto_const)?;
            pool.add(constant);
        }

        Ok(pool)
    }

    /// Convert our ConstantPool to protobuf ConstantPool
    fn convert_constant_pool_to_proto(pool: &ConstantPool) -> Result<proto::ConstantPool, String> {
        let mut constants = Vec::new();

        for i in 0..pool.len() {
            let constant = pool.get(i as u16)?;
            constants.push(Self::convert_constant_to_proto(constant)?);
        }

        Ok(proto::ConstantPool { constants })
    }

    /// Convert protobuf Constant to our Constant
    fn convert_constant(proto_const: proto::Constant) -> Result<Constant, String> {
        use proto::constant::Value as PV;

        let value = proto_const.value.ok_or("Constant has no value")?;

        Ok(match value {
            PV::IntValue(i) => Constant::Int(i),
            PV::FloatValue(f) => Constant::Float(f),
            PV::StringValue(s) => Constant::String(s),
            PV::BoolValue(b) => Constant::Bool(b),
            PV::NoneValue(_) => Constant::None,
            PV::Identifier(s) => Constant::Identifier(s),
            PV::FieldName(s) => Constant::FieldName(s),
            PV::FunctionName(s) => Constant::FunctionName(s),
        })
    }

    /// Convert our Constant to protobuf Constant
    fn convert_constant_to_proto(constant: &Constant) -> Result<proto::Constant, String> {
        use proto::constant::Value as PV;

        let value = match constant {
            Constant::Int(i) => PV::IntValue(*i),
            Constant::Float(f) => PV::FloatValue(*f),
            Constant::String(s) => PV::StringValue(s.clone()),
            Constant::Bool(b) => PV::BoolValue(*b),
            Constant::None => PV::NoneValue(proto::NoneValue {}),
            Constant::Identifier(s) => PV::Identifier(s.clone()),
            Constant::FieldName(s) => PV::FieldName(s.clone()),
            Constant::FunctionName(s) => PV::FunctionName(s.clone()),
        };

        Ok(proto::Constant { value: Some(value) })
    }
}

impl Probe {
    /// Convert from protobuf Probe message
    pub fn from_proto(proto: proto::Probe) -> Result<Self, String> {
        let spec = proto.spec.ok_or("Probe missing spec")?;
        let spec = ProbeSpec::from_proto(spec)?;

        Ok(Probe {
            id: proto.id,
            spec,
            predicate: proto.predicate,
            body: proto.body,
        })
    }

    /// Convert to protobuf Probe message
    pub fn to_proto(&self) -> Result<proto::Probe, String> {
        Ok(proto::Probe {
            id: self.id.clone(),
            spec: Some(self.spec.to_proto()?),
            predicate: self.predicate.clone(),
            body: self.body.clone(),
        })
    }
}

impl ProbeSpec {
    /// Convert from protobuf ProbeSpec message
    pub fn from_proto(proto: proto::ProbeSpec) -> Result<Self, String> {
        use proto::probe_spec::Spec;

        let spec = proto.spec.ok_or("ProbeSpec has no spec variant")?;

        match spec {
            Spec::Fn(fn_spec) => {
                let target = FnTarget::from_proto(fn_spec.target)?;
                Ok(ProbeSpec::Fn {
                    specifier: fn_spec.function_specifier,
                    target,
                })
            }
        }
    }

    /// Convert to protobuf ProbeSpec message
    pub fn to_proto(&self) -> Result<proto::ProbeSpec, String> {
        use proto::probe_spec::Spec;

        let spec = match self {
            ProbeSpec::Fn { specifier, target } => Spec::Fn(proto::FnProbeSpec {
                function_specifier: specifier.clone(),
                target: target.to_proto() as i32,
            }),
        };

        Ok(proto::ProbeSpec { spec: Some(spec) })
    }
}

impl FnTarget {
    /// Convert from protobuf FnProbeTarget enum
    pub fn from_proto(value: i32) -> Result<Self, String> {
        match proto::FnProbeTarget::try_from(value) {
            Ok(proto::FnProbeTarget::Entry) => Ok(FnTarget::Entry),
            Ok(proto::FnProbeTarget::Exit) => Ok(FnTarget::Exit),
            Err(_) => Err(format!("Invalid FnProbeTarget value: {}", value)),
        }
    }

    /// Convert to protobuf FnProbeTarget enum
    pub fn to_proto(self) -> proto::FnProbeTarget {
        match self {
            FnTarget::Entry => proto::FnProbeTarget::Entry,
            FnTarget::Exit => proto::FnProbeTarget::Exit,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_program_roundtrip() {
        let mut pool = ConstantPool::new();
        pool.add(Constant::Int(42));
        pool.add(Constant::String("test".to_string()));

        let program = Program {
            version: 1,
            constant_pool: pool,
            probes: vec![Probe {
                id: "test_probe".to_string(),
                spec: ProbeSpec::Fn {
                    specifier: "myapp.users.create".to_string(),
                    target: FnTarget::Entry,
                },
                predicate: vec![],
                body: vec![0x01, 0x00, 0x00], // PUSH_CONST 0
            }],
            sampling: 1.0,
        };

        // Convert to protobuf bytes
        let bytes = program.to_proto_bytes().unwrap();

        // Convert back
        let decoded = Program::from_proto_bytes(&bytes).unwrap();

        assert_eq!(decoded.version, 1);
        assert_eq!(decoded.sampling, 1.0);
        assert_eq!(decoded.probes.len(), 1);
        assert_eq!(decoded.probes[0].id, "test_probe");
    }
}
