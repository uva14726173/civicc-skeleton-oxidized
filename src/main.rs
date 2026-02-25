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

pub mod parser;

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

    /// Print the dot file of the AST
    #[arg(long, action)]
    print_dot_ast: bool,

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

    if args.print_dot_ast {
        println!("{}", ast::DOT_AST);
        return ExitCode::SUCCESS;
    }

    let Some(mut prog) = parser::parse_cpp(args.path, std::env::current_exe().unwrap().parent().unwrap().join("include")) else {return ExitCode::FAILURE;};

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
