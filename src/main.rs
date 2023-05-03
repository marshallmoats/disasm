use std::fs::File;
use std::io::prelude::*;
use std::iter::zip;
use std::path::Path;
use std::vec;

#[derive(Clone, Copy, Debug)]
enum IType {
    R,
    I,
    D,
    B,
    CB,
    IM,
    Pseudo,
    None,
}

#[derive(Debug)]
struct Instr {
    opcode: &'static str,
    instr_type: IType,
    rm: Option<u8>,
    rn: Option<u8>,
    rd: Option<u8>,
    rt: Option<u8>,
    imm: Option<i32>,
}

impl Instr {
    fn new(data: u32) -> Self {
        let (instr_type, opcode) = Instr::get_opcode(data);

        let sign_bit = match instr_type {
            IType::R => 15,
            IType::I => 21,
            IType::D => 20,
            IType::B => 25,
            IType::CB => 23,
            IType::IM => 20,
            _ => 0,
        };

        let last_bit = match instr_type {
            IType::R => 10,
            IType::I => 10,
            IType::D => 12,
            IType::B => 0,
            IType::CB => 5,
            IType::IM => 5,
            _ => 0,
        };

        let mut imm = ((data >> last_bit) & ((!0_u32).checked_shr(32 - sign_bit + last_bit)).unwrap_or(0)) as i32;
        if (1 << sign_bit) & data != 0 {
            imm |= !0 << (sign_bit - last_bit);
        }

        Instr {
            opcode,
            instr_type,
            rm: match instr_type {
                IType::R => Some(((data >> 16) & 31) as u8),
                _ => None,
            },
            rn: match instr_type {
                IType::R | IType::I | IType::D => Some(((data >> 5) & 31) as u8),
                _ => None,
            },
            rd: match instr_type {
                IType::R | IType::I | IType::IM | IType::Pseudo => Some((data & 31) as u8),
                _ => None,
            },
            rt: match instr_type {
                IType::D | IType::CB => Some((data & 31) as u8),
                _ => None,
            },
            imm: Some(imm),
        }
    }

    fn get_opcode(data: u32) -> (IType, &'static str) {
        let opcode = (data >> 21) & 2047;
        match opcode {
            0x0a0..=0x0bf => (IType::B, "B"),
            0x2a0..=0x2a7 => (IType::CB, "B."),
            0x450 => (IType::R, "AND"),
            0x458 => (IType::R, "ADD"),
            0x488..=0x489 => (IType::I, "ADDI"),
            0x490..=0x491 => (IType::I, "ANDI"),
            0x4a0..=0x4bf => (IType::B, "BL"),
            0x4d8 => (IType::R, "MUL"),
            0x550 => (IType::R, "ORR"),
            0x590..=0x591 => (IType::I, "ORRI"),
            0x5a0..=0x5a7 => (IType::CB, "CBZ"),
            0x5a8..=0x5af => (IType::CB, "CBNZ"),
            0x650 => (IType::R, "EOR"),
            0x658 => (IType::R, "SUB"),
            0x688..=0x689 => (IType::I, "SUBI"),
            0x690..=0x691 => (IType::I, "EORI"),
            0x69a => (IType::R, "LSR"),
            0x69b => (IType::R, "LSL"),
            0x6b0 => (IType::R, "BR"),
            0x758 => (IType::R, "SUBS"),
            0x788..=0x789 => (IType::I, "SUBIS"),
            0x794..=0x797 => (IType::IM, "MOVK"),
            0x7c0 => (IType::D, "STUR"),
            0x7c2 => (IType::D, "LDUR"),
            0x7fc => (IType::Pseudo, "PRNL"),
            0x7fd => (IType::Pseudo, "PRNT"),
            0x7fe => (IType::Pseudo, "DUMP"),
            0x7ff => (IType::Pseudo, "HALT"),
            _ => (IType::None, "Unknown"),
        }
    }
}

impl std::fmt::Display for Instr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.instr_type {
            IType::R => match self.opcode {
                "BR" => write!(f, "{} X{}", self.opcode, self.rn.unwrap()),
                "LSL" | "LSR" => write!(
                    f,
                    "{} X{}, X{}, #{}",
                    self.opcode,
                    self.rd.unwrap(),
                    self.rn.unwrap(),
                    self.imm.unwrap()
                ),
                _ => write!(
                    f,
                    "{} X{}, X{}, X{}",
                    self.opcode,
                    self.rd.unwrap(),
                    self.rn.unwrap(),
                    self.rm.unwrap()
                ),
            },
            IType::I => write!(
                f,
                "{} X{}, X{}, #{}",
                self.opcode,
                self.rd.unwrap(),
                self.rn.unwrap(),
                self.imm.unwrap()
            ),
            IType::D => write!(
                f,
                "{} X{}, [X{}, #{}]",
                self.opcode,
                self.rt.unwrap(),
                self.rn.unwrap(),
                self.imm.unwrap()
            ),
            IType::B => write!(f, "{} #{}", self.opcode, self.imm.unwrap()),
            IType::CB => write!(
                f,
                "{} X{}, #{}",
                self.opcode,
                self.rt.unwrap(),
                self.imm.unwrap()
            ),
            IType::IM => write!(
                f,
                "{} X{}, #{}",
                self.opcode,
                self.rd.unwrap(),
                self.imm.unwrap()
            ),
            IType::Pseudo => match self.opcode {
                "PRNT" => write!(f, "{} X{}", self.opcode, self.rd.unwrap()),
                _ => write!(f, "{}", self.opcode),
            },
            IType::None => write!(f, "{}", self.opcode),
        }
    }
}

fn main() {
    // get args
    let args = std::env::args().collect::<Vec<String>>();
    assert!(args.len() == 2, "Usage: {} <input file>", args[0]);

    // open input file
    let input_path = Path::new(&args[1]);
    let display = input_path.display();
    let mut input_file = match File::open(&input_path) {
        Err(why) => panic!("couldn't open {}: {}", display, why),
        Ok(file) => file,
    };

    // read input file
    let mut buf = Vec::new();
    input_file.read_to_end(&mut buf).unwrap();
    let mut instructions = Vec::new();
    for i in 0..buf.len() / 4 {
        let data = u32::from_be_bytes([buf[i * 4], buf[i * 4 + 1], buf[i * 4 + 2], buf[i * 4 + 3]]);
        instructions.push(Instr::new(data));
    }


    let mut labels: Vec<Option<String>> = vec![None; buf.len() / 4 + 1];
    let mut instrs: Vec<String> = vec![String::new(); buf.len() / 4 + 1];
    // last instruction is empty in case we have a branch to the end of the program
    instrs[buf.len() / 4] = String::new();

    for (i, instr) in instructions.iter().enumerate() {
        match instr.instr_type {
            IType::B | IType::CB => {
                let label_idx = (i as i32 + instr.imm.unwrap()) as usize;
                if let None = &labels[label_idx] {
                    labels[label_idx] = Some(String::from(format!("instr{}", label_idx)));
                }
            },
            _ => (),
        }

        match instr.opcode {
            "B" | "BL" => {
                let label_idx = (i as i32 + instr.imm.unwrap()) as usize;
                instrs[i] = format!("{} {}\n", instr.opcode, labels[label_idx].as_ref().unwrap());
            }
            "CBZ" | "CBNZ" => {
                let label_idx = (i as i32 + instr.imm.unwrap()) as usize;
                instrs[i] = format!("{} X{}, {}\n", instr.opcode, instr.rt.unwrap(), labels[label_idx].as_ref().unwrap());
            }
            "B." => {
                instrs[i] = format!(
                    "{}{} {}\n",
                    instr.opcode,
                    match instr.rt.unwrap() {
                        0x0 => "EQ",
                        0x1 => "NE",
                        0x2 => "HS",
                        0x3 => "LO",
                        0x4 => "MI",
                        0x5 => "PL",
                        0x6 => "VS",
                        0x7 => "VC",
                        0x8 => "HI",
                        0x9 => "LS",
                        0xa => "GE",
                        0xb => "LT",
                        0xc => "GT",
                        0xd => "LE",
                        other => panic!("Invalid conditional branch (B.cond): {}", other),
                    },
                    labels[(i as i32 + instr.imm.unwrap()) as usize].as_ref().unwrap()
                );
            }
            _ => instrs[i] = format!("{}\n", instr),
        }
    }

    let mut contents = String::new();
    for (label, instr) in zip(labels, instrs) {
        if let Some(l) = label {
            contents.push_str(format!("{}:\n", &l).as_str());
        }
        contents.push_str(&instr);
    }
    print!("{}", contents);
    std::fs::write("out.legv8asm", contents).unwrap();
}
