use traversal_generator_derive::generate_traversal;

pub struct NoTrav<T>(pub T);
#[generate_traversal]
pub mod ast {
    #[derive(Debug,PartialEq,Eq)]
    pub enum DyOpType {
        Add,
        Sub,
        Mul,
        Div,
        Mod,
        Lt,
        Le,
        Gt,
        Ge,
        Eq,
        Ne,
        And,
        Or,
    }
    /*
    #[derive(Debug)]
    pub enum UnOpType {
        Not, Neg
    }
    */

    #[derive(Debug)]
    pub struct Program(pub Vec<Assign>);

    #[derive(Debug)]
    pub struct Assign {
        pub varlet: String,
        pub expr: Expr,
    }

    #[derive(Debug)]
    pub enum Expr {
        Constant(Constant),
        Id(String),
        DyOp(DyOp),
    }

    #[derive(Debug)]
    pub struct DyOp {
        pub left: Box<Expr>,
        pub op: DyOpType,
        pub right: Box<Expr>,
    }

    #[derive(Debug)]
    pub enum Constant {
        Num(isize),
        Float(f64),
        Bool(bool),
    }
}

pub mod parser {
    use super::ast::*;
    use tree_sitter::{Parser, Node};
    //Could be done way more efficiently, but I can't be bothered rn

    pub fn node_to_string(n: Node, bytes: &[u8]) -> String {
        n.utf8_text(bytes).expect("Not UTF-8").to_string()
    }

    pub fn parse_expr(n: Node, bytes: &[u8]) -> Expr {
        match n.kind() {
            "constant" => Expr::Constant({
                let n = n.named_child(0).unwrap();
                let text = n.utf8_text(bytes).expect("Not UTF-8");
                match n.kind() {
                    "floatval" => Constant::Float(text.parse().unwrap()),
                    "intval" => Constant::Num(text.parse().unwrap()),
                    "boolval" => Constant::Bool(text == "true"),
                    _ => unreachable!(),
                }
            }),
            "id" => Expr::Id(node_to_string(n, bytes)),
            "dyop" => Expr::DyOp(DyOp{
                left: Box::new(parse_expr(n.child_by_field_name("left").unwrap(), bytes)),
                op: match n.child_by_field_name("op").unwrap().utf8_text(bytes).expect("Not UTF-8") {
                    "+" => DyOpType::Add,
                    "-" => DyOpType::Sub,
                    "*" => DyOpType::Mul,
                    "/" => DyOpType::Div,
                    "%" => DyOpType::Mod,
                    "<" => DyOpType::Lt,
                    "<=" => DyOpType::Le,
                    ">" => DyOpType::Gt,
                    ">=" => DyOpType::Ge,
                    "==" => DyOpType::Eq,
                    "!=" => DyOpType::Ne,
                    "&&" => DyOpType::And,
                    "||" => DyOpType::Or,
                    _ => unreachable!(),
                },
                right: Box::new(parse_expr(n.child_by_field_name("right").unwrap(), bytes)),
            }),
            _ => unreachable!(),
        }
    }
    pub fn parse_assign(n: Node, bytes: &[u8]) -> Assign {Assign{
        varlet: node_to_string(n.child_by_field_name("varlet").unwrap(), bytes), 
        expr: parse_expr(n.child_by_field_name("expr").unwrap(), bytes),
    }}
    pub fn parse_program(n: Node, bytes: &[u8]) -> Program {
        let mut cursor = n.walk();
        Program(n.children(&mut cursor).map(|c| parse_assign(c, bytes)).collect())
    }
    pub fn parse(bytes: &[u8]) -> Option<Program> {
        let mut parser = Parser::new();
        let lang = tree_sitter_civicc::LANGUAGE.into();
        parser.set_language(&lang).expect("Error loading grammar");
        let tree = parser.parse(bytes, None).unwrap();
        if print_errors(&tree, bytes) {return None;}
        Some(parse_program(tree.root_node(), bytes))
    }

    fn print_errors(tree: &tree_sitter::Tree, bytes: &[u8]) -> bool {
        let mut has_errors = false;
        let mut cursor = tree.walk();
        loop {
            let node = cursor.node();

            if node.is_error() || node.is_missing() {
                eprintln!("Syntax error at {}:{}", node.start_position().row + 1, node.start_position().column + 1);
                eprintln!("Problematic code: {}", str::from_utf8(&bytes[node.byte_range()]).expect("Invalit UTF-8"));
                has_errors = true;
            }

            if cursor.goto_first_child() {continue;}
            if cursor.goto_next_sibling() {continue;}

            loop {
                if !cursor.goto_parent() {return has_errors;}
                if cursor.goto_next_sibling() {break;}
            }
        }
    }
}

pub mod demo {
    use super::ast::*;

    pub fn rename_identifiers(prog: &mut Program) {
        prog.traversal_all(&mut |n| {
            if let Node::Expr(Expr::Id(id)) |
                   Node::Assign(Assign{varlet: id, ..}) = n {*id = format!("__{id}")}
        });
    }

    pub fn sum_ints(prog: &mut Program) {
        let mut sum = 0;
        prog.traversal_all(&mut |n| if let Node::Constant(Constant::Num(v)) = n {sum += *v;});
        println!("Sum of integers: {sum}");
    }
    
    pub fn opt_sub(prog: &mut Program) {
        prog.traversal_refrec(&|n, f| match n {
            Node::Expr(e) if matches!(e, Expr::DyOp(DyOp{op: DyOpType::Sub, ..})) => {
                if {
                    let Expr::DyOp(DyOp{left, right, ..}) = e else {unreachable!()};
                    left.traversal_ref(f);
                    right.traversal_ref(f);
                    (matches!((left.as_ref(),right.as_ref()), (Expr::Id(a), Expr::Id(b)) if a == b) ||
                     matches!((left.as_ref(),right.as_ref()), (Expr::Constant(Constant::Num(a)), Expr::Constant(Constant::Num(b))) if a == b))
                } {
                    *e = Expr::Constant(Constant::Num(0));
                }
                false
            },
            _ => true,
        });
    }
}


use clap::Parser;
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct CliArgs {
    /// Enable verbose mode.
    #[arg(short, long, action)]
    verbose: bool,

    /// Set a breakpoint.
    #[arg(short, long)]
    breakpoint: Option<usize>,

    /// Pretty print the structure of the compiler.
    #[arg(short, long, action)]
    structure: bool,

    /// The path to the civic file to read
    path: std::path::PathBuf,
}

use std::process::ExitCode;
fn main() -> ExitCode {
    let args = CliArgs::parse();

    let stages: Vec<(&str, &dyn Fn(&mut ast::Program))> = vec![
        ("RenameIdentifiers", &demo::rename_identifiers),
        ("SumInts", &demo::sum_ints),
        ("OptSubstraction", &demo::opt_sub),
    ];

    if args.structure {
        for (i, (name, _)) in stages.iter().enumerate() {
            println!("{i}: {name}");
        }
        return ExitCode::SUCCESS;
    }

    let Some(mut prog) = parser::parse(&std::fs::read(args.path).unwrap()) else {return ExitCode::FAILURE;};

    for (i, (name, f)) in stages.iter().enumerate() {
        if args.verbose {println!("Doing {name}");}
        f(&mut prog);
        if Some(i) == args.breakpoint {
            println!("{prog:#?}");
            return ExitCode::SUCCESS;
        }
    }

    println!("{prog:#?}");
    ExitCode::SUCCESS
}
