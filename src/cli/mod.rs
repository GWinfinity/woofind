//! CLI command implementations

pub mod commands;



/// Print a table to stdout
pub fn print_table(headers: &[&str], rows: &[Vec<String>]) {
    if rows.is_empty() {
        println!("No data to display.");
        return;
    }

    // Calculate column widths
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < widths.len() {
                widths[i] = widths[i].max(cell.len());
            }
        }
    }

    // Print header
    print!("  ");
    for (i, header) in headers.iter().enumerate() {
        print!("{:<width$}  ", header, width = widths[i]);
    }
    println!();

    // Print separator
    print!("  ");
    for (i, _) in headers.iter().enumerate() {
        print!("{:-<width$}  ", "", width = widths[i]);
    }
    println!();

    // Print rows
    for row in rows {
        print!("  ");
        for (i, cell) in row.iter().enumerate() {
            if i < widths.len() {
                print!("{:<width$}  ", cell, width = widths[i]);
            }
        }
        println!();
    }
}

/// Print a success message
pub fn print_success(msg: &str) {
    println!("✅ {}", msg);
}

/// Print an error message
pub fn print_error(msg: &str) {
    eprintln!("❌ {}", msg);
}

/// Print an info message
pub fn print_info(msg: &str) {
    println!("ℹ️  {}", msg);
}

/// Print a warning message
pub fn print_warning(msg: &str) {
    println!("⚠️  {}", msg);
}
