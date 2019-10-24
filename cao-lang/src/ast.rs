use crate::{Instruction, Value};
use arrayvec::{ArrayString, ArrayVec};
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;

/// Unique id of each nodes in a single compilation
pub type NodeId = i32;
/// Node by given id has inputs given by nodeids
/// Nodes may only have a finite amount of inputs
pub type Inputs = HashMap<NodeId, ArrayVec<[NodeId; 16]>>;
pub type Nodes = HashMap<NodeId, AstNode>;
/// Value of a node if any
pub type Values = HashMap<NodeId, Value>;
/// String of a node if any
pub type Strings = HashMap<NodeId, InputString>;

const INPUT_STR_LEN: usize = 128;
pub type InputString = ArrayString<[u8; INPUT_STR_LEN]>;

impl crate::ByteEncodeProperties for InputString {
    const BYTELEN: usize = INPUT_STR_LEN;

    fn encode(self) -> Vec<u8> {
        let mut rr = (self.len() as i32).encode();
        rr.extend(self.chars().map(|c| c as u8));
        rr
    }

    fn decode(bytes: &[u8]) -> Option<Self> {
        let len = i32::decode(bytes)?;
        let mut res = Self::new();
        for byte in bytes
            .iter()
            .skip(i32::BYTELEN)
            .take(len as usize)
            .map(|c| *c as char)
        {
            res.push(byte);
        }
        Some(res)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstNode {
    instruction: Instruction,
}

impl AstNode {
    pub fn new(instruction: Instruction) -> Self {
        Self { instruction }
    }
}

/// The accepted number of inputs of an instruction
/// None if unspecified
pub fn input_per_instruction(inst: Instruction) -> Option<u8> {
    use Instruction::*;
    match inst {
        AddInt | SubInt | AddFloat | SubFloat | Mul | MulFloat | Div | DivFloat => Some(2),
        LiteralInt | LiteralFloat | LiteralPtr | Pass | CopyLast => Some(0),
        Call | LiteralArray => None,
    }
}

/// Single unit of compilation, representing a single program
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationUnit {
    nodes: Nodes,
    inputs: Inputs,
    values: Values,
    strings: Strings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledProgram {
    pub leafid: NodeId,
    pub bytecode: Vec<u8>,
}

pub struct Compiler {
    unit: CompilationUnit,
}

impl Compiler {
    pub fn compile(unit: CompilationUnit) -> Result<Vec<CompiledProgram>, String> {
        if unit.nodes.is_empty() {
            return Err("Can not compile program with no entry point!".to_owned());
        }
        let mut compiler = Compiler { unit };
        let todo: Vec<NodeId> = compiler
            .unit
            .nodes
            .iter()
            .map(|(k, _)| k)
            .filter(|k| {
                for it in compiler.unit.inputs.values() {
                    for n in it {
                        if n == *k {
                            return false;
                        }
                    }
                }
                true
            })
            .cloned()
            .collect();

        let mut compiled_programs = Vec::with_capacity(4);
        for nodeid in todo.into_iter() {
            let program = compiler.compile_node(nodeid)?;
            compiled_programs.push(program);
        }

        Ok(compiled_programs)
    }

    fn compile_node(&mut self, node: NodeId) -> Result<CompiledProgram, String> {
        let mut compiled = CompiledProgram {
            bytecode: Vec::new(),
            leafid: node,
        };
        self.process_node(node, &mut compiled.bytecode)?;

        Ok(compiled)
    }

    fn process_node(&mut self, nodeid: NodeId, bytes: &mut Vec<u8>) -> Result<(), String> {
        use crate::traits::ByteEncodeProperties;
        use Instruction::*;

        Compiler::validate_node(nodeid, &mut self.unit)?;

        if let Some(inputs) = self.unit.inputs.get(&nodeid) {
            for nodeid in inputs.clone().into_iter() {
                self.process_node(nodeid, bytes)?;
            }
        }
        let node = &self.unit.nodes[&nodeid];
        match node.instruction {
            Call => {
                bytes.push(node.instruction as u8);
                bytes.append(&mut self.unit.strings[&nodeid].encode());
            }
            LiteralArray | LiteralPtr | LiteralFloat | LiteralInt => {
                bytes.push(node.instruction as u8);
                match (node.instruction, self.unit.values[&nodeid]) {
                    (Instruction::LiteralInt, Value::IValue(v)) => {
                        bytes.append(&mut v.encode());
                    }
                    (Instruction::LiteralFloat, Value::FValue(v)) => {
                        bytes.append(&mut v.encode());
                    }
                    (Instruction::LiteralPtr, Value::Pointer(v)) => {
                        bytes.append(&mut v.encode());
                    }
                    (Instruction::LiteralArray, Value::IValue(v)) => {
                        bytes.append(&mut v.encode());
                    }
                    _ => panic!(
                        "Literal {:?} got invalid value {:?}",
                        node.instruction, self.unit.values[&nodeid]
                    ),
                }
            }
            _ => bytes.push(node.instruction as u8),
        }
        Ok(())
    }

    pub fn validate_node(node: NodeId, cu: &CompilationUnit) -> Result<(), String> {
        if let Some(n) = input_per_instruction(cu.nodes[&node].instruction) {
            let n_inputs = cu.inputs.get(&node).map(|x| x.len()).unwrap_or(0)
                + cu.strings.get(&node).map(|_| 1).unwrap_or(0);
            if n_inputs != n as usize {
                return Err(format!(
                    "Invalid number of inputs, expected {} got {}",
                    n, n_inputs
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::VM;

    #[test]
    fn test_compiling_simple_program() {
        let nodes: Nodes = [
            (
                0,
                AstNode {
                    instruction: Instruction::LiteralFloat,
                },
            ),
            (
                1,
                AstNode {
                    instruction: Instruction::LiteralFloat,
                },
            ),
            (
                2,
                AstNode {
                    instruction: Instruction::AddFloat,
                },
            ),
        ]
        .into_iter()
        .cloned()
        .collect();
        let values: Values = [(0, Value::FValue(42.0)), (1, Value::FValue(512.0))]
            .into_iter()
            .map(|x| *x)
            .collect();
        let inputs: Inputs = [(2, [0, 1].into_iter().cloned().collect())]
            .into_iter()
            .cloned()
            .collect();
        let strings: Strings = [].into_iter().cloned().collect();
        let program = CompilationUnit {
            nodes,
            values,
            inputs,
            strings,
        };

        let programs = Compiler::compile(program).unwrap();
        assert_eq!(programs.len(), 1);
        let program = &programs[0];
        assert_eq!(program.leafid, 2);

        println!("{:?}", program);

        // Compilation was successful

        let mut vm = VM::new();
        vm.run(&program.bytecode).unwrap();

        assert_eq!(vm.stack.len(), 1);

        let value = vm.stack.last().unwrap();
        match value {
            Value::FValue(i) => assert_eq!(*i, 42.0 + 512.0),
            _ => panic!("Invalid value in the stack"),
        }
    }
}