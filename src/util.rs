pub(crate) fn print_bytes(buf: &[u8], width: usize) {
    let mut chars = String::with_capacity(width);
    let mut bytes = String::with_capacity(width * 3);

    for (i, b) in buf.iter().enumerate() {
        let c = *b as char;

        if c.is_ascii_graphic() {
            chars.push(c);
        } else {
            chars.push('.');
        }

        bytes.push_str(&format!("{:02X}", c as u8));

        if (i + 1) % width == 0 {
            info!("{bytes}    {chars}");
            chars.clear();
            bytes.clear();
        } else {
            bytes.push(' ');
        }
    }

    if buf.len() % width != 0 {
        info!("{bytes}    {chars}");
    }
}
