//! Program representation and protobuf serialization for HogTrace VM.
//!
//! This module defines the compiled program structure that can be executed by the VM
//! and provides serialization/deserialization via Protocol Buffers.

use std::hash::{DefaultHasher, Hash, Hasher};

use crate::constant_pool::{Constant, ConstantPool};

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/hogtrace.vm.rs"));
}

#[derive(Debug, Clone)]
pub struct ProgramList {
    pub programs: Vec<HogTraceProgram>,
    pub retrieved_at: i64,
}

#[derive(Debug, Clone)]
pub struct HogTraceProgram {
    pub id: String,
    pub sampling: f32,
    pub hash: String,
    pub limit: u32,
    pub compiled_program: CompiledProgram,
}

#[derive(Debug, Clone)]
pub struct CompiledProgram {
    pub bytecode_version: u32,
    pub constant_pool: ConstantPool,
    pub probes: Vec<Probe>,
}

/// This hash is deterministic
/// This is so even if you reorder the probes or constant pool you'll
/// end up with the same hash. This is used on the client side to avoid
/// reinstalling the program if nothing changed.
impl Hash for CompiledProgram {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // We first pre-hash probes and constants, then we sort them.
        // This ensures that when you have the same probes but different order that
        // we will get the same final hash.

        let mut probe_hashes = Vec::with_capacity(self.probes.len());

        for probe in &self.probes {
            let mut hasher = DefaultHasher::new();
            probe.hash(&mut hasher);
            probe_hashes.push(hasher.finish())
        }

        let mut constant_hashes = Vec::with_capacity(self.constant_pool.constants.len());

        for constant in &self.constant_pool.constants {
            let mut hasher = DefaultHasher::new();
            constant.hash(&mut hasher);
            constant_hashes.push(hasher.finish());
        }

        probe_hashes.sort();
        constant_hashes.sort();

        for prehash in probe_hashes.drain(..).chain(constant_hashes.drain(..)) {
            prehash.hash(state)
        }
    }
}

#[derive(Debug, Clone, Hash)]
pub struct Probe {
    pub id: String,
    pub spec: ProbeSpec,
    pub predicate: Vec<u8>,
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, Hash)]
pub enum ProbeSpec {
    Fn { specifier: String, target: FnTarget },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FnTarget {
    Entry,
    Exit,
}

impl ProgramList {
    pub fn from_proto_bytes(bytes: &[u8]) -> Result<Self, String> {
        use prost::Message;

        let proto_programs = proto::ProgramList::decode(bytes)
            .map_err(|e| format!("Failed to decode protobuf: {}", e))?;

        Self::from_proto(proto_programs)
    }

    pub fn from_proto(proto: proto::ProgramList) -> Result<Self, String> {
        let programs = proto
            .programs
            .into_iter()
            .map(HogTraceProgram::from_proto)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(ProgramList {
            programs: programs,
            retrieved_at: proto.retrieved_at,
        })
    }

    pub fn to_proto_bytes(&self) -> Result<Vec<u8>, String> {
        use prost::Message;

        let proto = self.to_proto()?;
        let mut buf = Vec::new();
        proto
            .encode(&mut buf)
            .map_err(|e| format!("Failed to encode protobuf: {}", e))?;
        Ok(buf)
    }

    pub fn to_proto(&self) -> Result<proto::ProgramList, String> {
        Ok(proto::ProgramList {
            programs: self
                .programs
                .iter()
                .map(HogTraceProgram::to_proto)
                .collect::<Result<Vec<_>, _>>()?,
            retrieved_at: self.retrieved_at,
        })
    }
}

impl HogTraceProgram {
    pub fn from_proto_bytes(bytes: &[u8]) -> Result<Self, String> {
        use prost::Message;

        let proto_htp = proto::HogTraceProgram::decode(bytes)
            .map_err(|e| format!("Failed to decode protobuf: {}", e))?;

        Self::from_proto(proto_htp)
    }

    pub fn from_proto(proto: proto::HogTraceProgram) -> Result<Self, String> {
        // TODO(Marce): Why is compiled program an Option?
        let compiled_program = CompiledProgram::from_proto(proto.compiled_program.unwrap())?;

        Ok(Self {
            id: proto.id,
            sampling: proto.sampling,
            hash: proto.hash,
            limit: proto.limit,
            compiled_program: compiled_program,
        })
    }

    pub fn to_proto_bytes(&self) -> Result<Vec<u8>, String> {
        use prost::Message;

        let proto = self.to_proto()?;
        let mut buf = Vec::new();
        proto
            .encode(&mut buf)
            .map_err(|e| format!("Failed to encode protobuf: {}", e))?;
        Ok(buf)
    }

    pub fn to_proto(&self) -> Result<proto::HogTraceProgram, String> {
        Ok(proto::HogTraceProgram {
            id: self.id.clone(),
            hash: self.hash.clone(),
            sampling: self.sampling,
            limit: self.limit,
            compiled_program: Some(CompiledProgram::to_proto(&self.compiled_program)?),
        })
    }
}

impl CompiledProgram {
    pub fn from_compilation(probes: Vec<Probe>, constant_pool: ConstantPool) -> Self {
        Self {
            bytecode_version: 1,
            constant_pool,
            probes,
        }
    }

    /// Deserialize a Program from protobuf bytes
    pub fn from_proto_bytes(bytes: &[u8]) -> Result<Self, String> {
        use prost::Message;

        let proto_program = proto::CompiledProgram::decode(bytes)
            .map_err(|e| format!("Failed to decode protobuf: {}", e))?;

        Self::from_proto(proto_program)
    }

    /// Convert from protobuf Program message
    pub fn from_proto(proto: proto::CompiledProgram) -> Result<Self, String> {
        let constant_pool = Self::convert_constant_pool(proto.constant_pool)?;
        let probes = proto
            .probes
            .into_iter()
            .map(Probe::from_proto)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(CompiledProgram {
            bytecode_version: proto.bytecode_version,
            constant_pool,
            probes,
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
    pub fn to_proto(&self) -> Result<proto::CompiledProgram, String> {
        Ok(proto::CompiledProgram {
            bytecode_version: self.bytecode_version,
            constant_pool: Some(Self::convert_constant_pool_to_proto(&self.constant_pool)?),
            probes: self
                .probes
                .iter()
                .map(Probe::to_proto)
                .collect::<Result<Vec<_>, _>>()?,
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

    fn hash_something(v: &impl Hash) -> u64 {
        let mut h = DefaultHasher::new();
        v.hash(&mut h);
        h.finish()
    }

    #[test]
    fn test_program_roundtrip() {
        let mut pool = ConstantPool::new();
        pool.add(Constant::Int(42));
        pool.add(Constant::String("test".to_string()));

        let program = CompiledProgram {
            bytecode_version: 1,
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
        };

        // Convert to protobuf bytes
        let bytes = program.to_proto_bytes().unwrap();

        // Convert back
        let decoded = CompiledProgram::from_proto_bytes(&bytes).unwrap();

        assert_eq!(decoded.bytecode_version, 1);
        assert_eq!(decoded.probes.len(), 1);
        assert_eq!(decoded.probes[0].id, "test_probe");
    }

    #[test]
    fn test_hash() {
        let mut pool = ConstantPool::new();
        pool.add(Constant::Int(42));
        pool.add(Constant::String("test".to_string()));
        pool.add(Constant::Float(4.2));

        let probe_a = Probe {
            id: "test_probe".to_string(),
            spec: ProbeSpec::Fn {
                specifier: "myapp.users.create".to_string(),
                target: FnTarget::Entry,
            },
            predicate: vec![],
            body: vec![0x01, 0x00, 0x00],
        };
        let probe_b = Probe {
            id: "test_probe_2".to_string(),
            spec: ProbeSpec::Fn {
                specifier: "myapp.users.delete".to_string(),
                target: FnTarget::Entry,
            },
            predicate: vec![],
            body: vec![0x01, 0x00, 0x10],
        };

        let program = CompiledProgram {
            bytecode_version: 1,
            constant_pool: pool.clone(),
            probes: vec![probe_a.clone(), probe_b.clone()],
        };

        assert_eq!(hash_something(&program), hash_something(&program));

        let shuffled_program = CompiledProgram {
            bytecode_version: 1,
            constant_pool: pool,
            probes: vec![probe_b.clone(), probe_a.clone()],
        };

        assert_eq!(hash_something(&program), hash_something(&shuffled_program),);
    }
}
