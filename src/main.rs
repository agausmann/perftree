use std::io;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

fn main() {
    let mock_diff = Diff {
        node: (200, 250),
        children: vec![
            ("a2a3".to_string(), Some(80), Some(72)),
            ("a2a4".to_string(), Some(60), Some(60)),
            ("b3b4".to_string(), None, Some(65)),
        ],
    };

    let mut write = StandardStream::stdout(ColorChoice::Auto);
    mock_diff.write_colored(&mut write).unwrap();
}

#[derive(Debug, Clone)]
struct Diff {
    node: (u128, u128),
    children: Vec<(String, Option<u128>, Option<u128>)>,
}

impl Diff {
    fn write_colored<W>(&self, mut write: W) -> io::Result<()>
    where
        W: WriteColor,
    {
        for &(ref move_, lhs, rhs) in &self.children {
            if lhs != rhs {
                write.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true))?;
            }
            write!(write, "{}", move_)?;
            if let Some(lhs) = lhs {
                write!(write, " {:>20}", lhs)?;
            } else {
                write!(write, " {:>20}", "")?;
            }
            if let Some(rhs) = rhs {
                write!(write, " {:>20}", rhs)?;
            } else {
                write!(write, " {:>20}", "")?;
            }
            writeln!(write)?;
            write.reset()?;
        }
        Ok(())
    }
}
