/// Bytecode instruction set for the HogTrace VM
///
/// This is a stack-based VM with no control flow (no jumps, no conditionals).
/// Execution proceeds linearly from start to end, with the final stack value as the result.

/// Opcodes are single bytes (u8) for compact representation
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opcode {
    // Stack operations
    /// Push constant from constant pool onto stack
    /// Operand: u16 (constant pool index)
    PushConst = 0x01,

    /// Pop top value from stack and discard
    Pop = 0x02,

    /// Duplicate top value on stack
    Dup = 0x03,

    // Variable operations
    /// Load variable by name (identifier from constant pool)
    /// Examples: args, arg0, arg1, retval, exception, kwargs, self, locals, globals
    /// Operand: u16 (constant pool index for identifier)
    LoadVar = 0x10,

    /// Store top of stack to variable (for request-scoped variables like $req.user_id)
    /// Operand: u16 (constant pool index for identifier)
    StoreVar = 0x11,

    // Arithmetic operators (binary, pop 2, push 1)
    Add = 0x20,      // a + b
    Sub = 0x21,      // a - b
    Mul = 0x22,      // a * b
    Div = 0x23,      // a / b
    Mod = 0x24,      // a % b

    // Comparison operators (binary, pop 2, push 1 bool)
    Eq = 0x30,       // a == b
    Ne = 0x31,       // a != b
    Lt = 0x32,       // a < b
    Gt = 0x33,       // a > b
    Le = 0x34,       // a <= b
    Ge = 0x35,       // a >= b

    // Logical operators
    And = 0x40,      // a && b (binary, pop 2, push 1)
    Or = 0x41,       // a || b (binary, pop 2, push 1)
    Not = 0x42,      // !a (unary, pop 1, push 1)

    // Field/attribute access
    /// Get attribute from object: obj.field
    /// Operand: u16 (constant pool index for field name)
    /// Stack: [obj] -> [value]
    GetAttr = 0x50,

    /// Get item from object: obj[key]
    /// No operand - key is on stack
    /// Stack: [obj, key] -> [value]
    GetItem = 0x51,

    /// Set attribute on object: obj.field = value
    /// Operand: u16 (constant pool index for field name)
    /// Stack: [obj, value] -> []
    SetAttr = 0x52,

    // Function calls
    /// Call function with N arguments
    /// Operands: u16 (constant pool index for function name), u8 (arg count)
    /// Stack: [arg0, arg1, ..., argN] -> [result]
    /// Examples: timestamp(), rand(), len(args), capture(args, retval)
    CallFunc = 0x60,
}

impl Opcode {
    /// Try to parse a u8 into an Opcode
    pub fn from_u8(byte: u8) -> Result<Self, String> {
        match byte {
            0x01 => Ok(Opcode::PushConst),
            0x02 => Ok(Opcode::Pop),
            0x03 => Ok(Opcode::Dup),
            0x10 => Ok(Opcode::LoadVar),
            0x11 => Ok(Opcode::StoreVar),
            0x20 => Ok(Opcode::Add),
            0x21 => Ok(Opcode::Sub),
            0x22 => Ok(Opcode::Mul),
            0x23 => Ok(Opcode::Div),
            0x24 => Ok(Opcode::Mod),
            0x30 => Ok(Opcode::Eq),
            0x31 => Ok(Opcode::Ne),
            0x32 => Ok(Opcode::Lt),
            0x33 => Ok(Opcode::Gt),
            0x34 => Ok(Opcode::Le),
            0x35 => Ok(Opcode::Ge),
            0x40 => Ok(Opcode::And),
            0x41 => Ok(Opcode::Or),
            0x42 => Ok(Opcode::Not),
            0x50 => Ok(Opcode::GetAttr),
            0x51 => Ok(Opcode::GetItem),
            0x52 => Ok(Opcode::SetAttr),
            0x60 => Ok(Opcode::CallFunc),
            _ => Err(format!("Unknown opcode: 0x{:02x}", byte)),
        }
    }

    /// Returns the number of operand bytes this opcode requires
    pub fn operand_size(&self) -> usize {
        match self {
            // Opcodes with u16 operand (2 bytes)
            Opcode::PushConst | Opcode::LoadVar | Opcode::StoreVar | Opcode::GetAttr | Opcode::SetAttr => 2,

            // CallFunc has u16 + u8 (3 bytes total)
            Opcode::CallFunc => 3,

            // All other opcodes have no operands
            _ => 0,
        }
    }
}

/// Helper to read a u16 from bytecode in little-endian format
#[inline]
pub fn read_u16(bytecode: &[u8], offset: usize) -> Result<u16, String> {
    if offset + 2 > bytecode.len() {
        return Err("Unexpected end of bytecode while reading u16".to_string());
    }
    Ok(u16::from_le_bytes([bytecode[offset], bytecode[offset + 1]]))
}

/// Helper to read a u8 from bytecode
#[inline]
pub fn read_u8(bytecode: &[u8], offset: usize) -> Result<u8, String> {
    if offset >= bytecode.len() {
        return Err("Unexpected end of bytecode while reading u8".to_string());
    }
    Ok(bytecode[offset])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opcode_roundtrip() {
        let opcodes = [
            Opcode::PushConst,
            Opcode::Add,
            Opcode::CallFunc,
            Opcode::GetAttr,
        ];

        for opcode in opcodes {
            let byte = opcode as u8;
            let parsed = Opcode::from_u8(byte).unwrap();
            assert_eq!(opcode, parsed);
        }
    }

    #[test]
    fn test_invalid_opcode() {
        assert!(Opcode::from_u8(0xFF).is_err());
    }

    #[test]
    fn test_operand_sizes() {
        assert_eq!(Opcode::PushConst.operand_size(), 2);
        assert_eq!(Opcode::CallFunc.operand_size(), 3);
        assert_eq!(Opcode::Add.operand_size(), 0);
    }
}
