use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct ReceiptItemData {
    pub name: String,
    pub sku: String,
    pub qty: i32,
    pub unit_price_cents: i32,
    pub line_total_cents: i32,
}

#[derive(Clone, Debug)]
pub struct ReceiptData {
    pub merchant_name: String,
    pub merchant_address: String,
    pub merchant_phone: String,
    pub order_no: String,
    pub timestamp: String,
    pub cashier_name: String,
    pub items: Vec<ReceiptItemData>,
    pub subtotal_cents: i32,
    pub tax_cents: i32,
    pub discount_cents: i32,
    pub total_cents: i32,
    pub payment_method: String,
    pub tendered_cents: i32,
    pub change_cents: i32,
}

fn format_money(cents: i32) -> String {
    format!("${}.{:02}", cents / 100, (cents % 100).abs())
}

fn center_text(text: &str, width: usize) -> String {
    if text.len() >= width {
        return text[..width].to_string();
    }
    let left_pad = (width - text.len()) / 2;
    let right_pad = width - text.len() - left_pad;
    format!("{}{}{}", " ".repeat(left_pad), text, " ".repeat(right_pad))
}

fn left_right_align(left: &str, right: &str, width: usize) -> String {
    let combined_len = left.len() + right.len();
    if combined_len >= width {
        return format!("{} {}", left, right);
    }
    let spaces = width - combined_len;
    format!("{}{}{}", left, " ".repeat(spaces), right)
}

/// Generates a clean 80mm industrial monospaced receipt (48 columns standard)
pub fn generate_ascii_receipt(data: &ReceiptData, width: usize) -> String {
    let mut out = String::new();
    let divider = "-".repeat(width);
    let double_divider = "=".repeat(width);

    // 1. Merchant Header
    out.push_str(&center_text(&data.merchant_name, width));
    out.push('\n');
    out.push_str(&center_text(&data.merchant_address, width));
    out.push('\n');
    out.push_str(&center_text(&data.merchant_phone, width));
    out.push('\n');
    out.push_str(&divider);
    out.push('\n');

    // 2. Metadata
    out.push_str(&left_right_align(
        &format!("ORDER: {}", data.order_no),
        &data.timestamp,
        width,
    ));
    out.push('\n');
    out.push_str(&left_right_align(
        &format!("STAFF: {}", data.cashier_name),
        "TERMINAL #01",
        width,
    ));
    out.push('\n');
    out.push_str(&divider);
    out.push('\n');

    // 3. Item List Header
    out.push_str(&left_right_align("QTY  ITEM DESCRIPTION", "TOTAL", width));
    out.push('\n');
    out.push_str(&divider);
    out.push('\n');

    // 4. Items
    for item in &data.items {
        let left_part = format!("{}x  {}", item.qty, item.name);
        let right_part = format_money(item.line_total_cents);
        out.push_str(&left_right_align(&left_part, &right_part, width));
        out.push('\n');
        out.push_str(&format!(
            "     {} @ {} each\n",
            item.sku,
            format_money(item.unit_price_cents)
        ));
    }
    out.push_str(&divider);
    out.push('\n');

    // 5. Financial Summary
    out.push_str(&left_right_align(
        "Subtotal:",
        &format_money(data.subtotal_cents),
        width,
    ));
    out.push('\n');
    out.push_str(&left_right_align(
        "Sales Tax (8.25%):",
        &format_money(data.tax_cents),
        width,
    ));
    out.push('\n');

    if data.discount_cents > 0 {
        out.push_str(&left_right_align(
            "Discounts Applied:",
            &format!("-{}", format_money(data.discount_cents)),
            width,
        ));
        out.push('\n');
    }

    out.push_str(&double_divider);
    out.push('\n');
    out.push_str(&left_right_align(
        "TOTAL AMOUNT:",
        &format_money(data.total_cents),
        width,
    ));
    out.push('\n');
    out.push_str(&double_divider);
    out.push('\n');

    // 6. Payment Breakdown
    out.push_str(&left_right_align(
        &format!("Payment ({}):", data.payment_method),
        &format_money(data.tendered_cents),
        width,
    ));
    out.push('\n');
    out.push_str(&left_right_align(
        "Change Returned:",
        &format_money(data.change_cents),
        width,
    ));
    out.push('\n');
    out.push_str(&divider);
    out.push('\n');

    // 7. Simulated Barcode & Footer
    out.push_str(&center_text(&format!("*{}*", data.order_no), width));
    out.push('\n');
    out.push_str(&center_text("||| || ||||| | |||| ||| ||||", width));
    out.push('\n');
    out.push('\n');
    out.push_str(&center_text(
        "100% Pure Native Rust • Direct3D GPU Engine",
        width,
    ));
    out.push('\n');
    out.push_str(&center_text("Thank you for your business!", width));
    out.push('\n');

    out
}

/// Generates standard ESC/POS binary command stream for direct hardware thermal printers
pub fn generate_esc_pos_bytes(data: &ReceiptData) -> Vec<u8> {
    let mut bytes = Vec::new();

    // 1. ESC @ : Initialize Printer
    bytes.extend_from_slice(&[0x1B, 0x40]);

    // 2. ESC a 1 : Center Alignment
    bytes.extend_from_slice(&[0x1B, 0x61, 0x01]);

    // 3. ESC E 1 : Bold On
    bytes.extend_from_slice(&[0x1B, 0x45, 0x01]);
    bytes.extend_from_slice(data.merchant_name.as_bytes());
    bytes.push(b'\n');

    // ESC E 0 : Bold Off
    bytes.extend_from_slice(&[0x1B, 0x45, 0x00]);
    bytes.extend_from_slice(data.merchant_address.as_bytes());
    bytes.push(b'\n');
    bytes.extend_from_slice(data.merchant_phone.as_bytes());
    bytes.extend_from_slice(b"\n------------------------------------------------\n");

    // 4. ESC a 0 : Left Alignment for Items & Totals
    bytes.extend_from_slice(&[0x1B, 0x61, 0x00]);
    let ascii_content = generate_ascii_receipt(data, 48);
    bytes.extend_from_slice(ascii_content.as_bytes());

    // 5. Line feeds for paper advance
    bytes.extend_from_slice(b"\n\n\n");

    // 6. GS V 0 : Full Automatic Guillotine Paper Cut
    bytes.extend_from_slice(&[0x1D, 0x56, 0x00]);

    bytes
}

/// Spools and saves the receipt to the local file system (`receipts/` directory)
pub fn save_receipt_to_disk(data: &ReceiptData) -> Result<PathBuf, String> {
    let receipts_dir = Path::new("receipts");
    if !receipts_dir.exists() {
        fs::create_dir_all(receipts_dir)
            .map_err(|e| format!("Failed to create receipts directory: {}", e))?;
    }

    let file_path = receipts_dir.join(format!("{}.txt", data.order_no));
    let raw_text = generate_ascii_receipt(data, 48);

    fs::write(&file_path, raw_text)
        .map_err(|e| format!("Failed to write receipt file: {}", e))?;

    // Also write binary ESC/POS stream
    let prn_path = receipts_dir.join(format!("{}.prn", data.order_no));
    let esc_bytes = generate_esc_pos_bytes(data);
    let _ = fs::write(prn_path, esc_bytes);

    Ok(file_path)
}

/// Dispatches print job: writes receipt to disk and executes system print/viewer
pub fn spool_receipt_print(data: &ReceiptData) -> Result<PathBuf, String> {
    let path = save_receipt_to_disk(data)?;

    // On Windows, launch the generated receipt in notepad / default viewer
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/c", "start", "", &path.to_string_lossy()])
            .spawn();
    }

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_receipt() -> ReceiptData {
        ReceiptData {
            merchant_name: "TEST CAFE & ROASTER".to_string(),
            merchant_address: "123 Rust Way".to_string(),
            merchant_phone: "555-0100".to_string(),
            order_no: "ORD-9999".to_string(),
            timestamp: "2026-10-31 15:00:00".to_string(),
            cashier_name: "Tester".to_string(),
            items: vec![
                ReceiptItemData {
                    name: "Cold Brew".to_string(),
                    sku: "BEV-01".to_string(),
                    qty: 2,
                    unit_price_cents: 400,
                    line_total_cents: 800,
                },
                ReceiptItemData {
                    name: "Croissant".to_string(),
                    sku: "BAK-01".to_string(),
                    qty: 1,
                    unit_price_cents: 350,
                    line_total_cents: 350,
                },
            ],
            subtotal_cents: 1150,
            tax_cents: 95,
            discount_cents: 0,
            total_cents: 1245,
            payment_method: "CASH".to_string(),
            tendered_cents: 2000,
            change_cents: 755,
        }
    }

    #[test]
    fn test_ascii_receipt_generation() {
        let r = create_test_receipt();
        let txt = generate_ascii_receipt(&r, 48);
        assert!(txt.contains("TEST CAFE & ROASTER"));
        assert!(txt.contains("ORD-9999"));
        assert!(txt.contains("Cold Brew"));
        assert!(txt.contains("$12.45"));
        assert!(txt.contains("Change Returned:"));
    }

    #[test]
    fn test_esc_pos_bytes() {
        let r = create_test_receipt();
        let bytes = generate_esc_pos_bytes(&r);
        assert!(!bytes.is_empty());
        // Verify ESC @ (init) and GS V 0 (cut)
        assert_eq!(&bytes[0..2], &[0x1B, 0x40]);
        assert_eq!(&bytes[bytes.len() - 3..], &[0x1D, 0x56, 0x00]);
    }
}

