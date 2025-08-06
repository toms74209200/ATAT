use std::io::{self, Write};

pub fn println(message: &str, writer: &mut Option<&mut dyn Write>) -> io::Result<()> {
    if let Err(e) = writeln!(io::stdout(), "{message}") {
        eprintln!("標準出力への書き込みに失敗しました: {e}");
    }

    if let Some(w) = writer {
        writeln!(w, "{message}")?;
    }

    Ok(())
}
