use rust_xlsxwriter::{ColNum, Format, RowNum, Worksheet};

/// Поставщикs
pub fn write_vendor_name(
    organization_info: &Vec<(String, u32, f64, f64, f64, f64)>,
    worksheet: &mut Worksheet,
    mut row: RowNum,
    col: ColNum,
) {
    organization_info.iter().for_each(|record| {
        worksheet.write_string(row, col, record.0.as_str()).unwrap();
        row += 1;
    });
}

/// кол-во
pub fn write_number_of_transactions_per_day(
    organization_info: &Vec<(String, u32, f64, f64, f64, f64)>,
    worksheet: &mut Worksheet,
    mut row: RowNum,
    col: ColNum,
) -> u32 {
    let mut all_transaction: u32 = 0;
    organization_info.iter().for_each(|record| {
        worksheet.write_number(row, col, record.1.clone()).unwrap();
        row += 1;
        all_transaction += record.1
    });
    all_transaction
}

/// Сумма
pub fn write_number_of_amount_per_day(
    organization_info: &Vec<(String, u32, f64, f64, f64, f64)>,
    worksheet: &mut Worksheet,
    mut row: RowNum,
    col: ColNum,
) -> f64 {
    let mut all_amount: f64 = 0.0;
    organization_info.iter().for_each(|record| {
        worksheet.write_number(row, col, record.2).unwrap();
        row += 1;
        let format_number = format!("{:.3}", record.2.to_string().parse::<f64>().unwrap_or(0.0));
        all_amount += format_number.parse::<f64>().unwrap_or(0.0);
    });

    all_amount
}

/// Сумма комиссии с Плательщика
pub fn write_number_of_commission_per_day(
    organization_info: &Vec<(String, u32, f64, f64, f64, f64)>,
    worksheet: &mut Worksheet,
    mut row: RowNum,
    col: ColNum,
) {
    organization_info.iter().for_each(|record| {
        let format_number = format!("{:.3}", record.3);
        worksheet.write_number(row, col, format_number.parse::<f64>().unwrap()).unwrap();
        row += 1;
    });
}

pub fn write_vendor_info(
    vendors_info: &Vec<(String, u32, f64, f64, f64, f64)>,
    worksheet: &mut Worksheet, mut row: RowNum, mut col: ColNum
) -> (RowNum, ColNum) {
    let format_bold = Format::new().set_bold();
    fn format_number(number: f64) -> f64 {
        format!("{:.3}", number).parse::<f64>().unwrap()
    }

    vendors_info.iter().for_each(|vendor_info| {
        worksheet
            .write_string_with_format(
                row, col,
                vendor_info.0.as_str(), &format_bold
            ).unwrap();
        col += 1;
        worksheet
            .write_number_with_format(
                row, col,
                vendor_info.1, &format_bold
            ).unwrap();
        col += 1;
        worksheet
            .write_number_with_format(
                row, col,
                format_number(vendor_info.2), &format_bold
            ).unwrap();
        col += 1;
        worksheet
            .write_number_with_format(
                row, col,
                format_number(vendor_info.3), &format_bold
            ).unwrap();
        col += 1;
        worksheet
            .write_number_with_format(
                row, col,
                format_number(vendor_info.4), &format_bold
            ).unwrap();
        col += 1;
        worksheet
            .write_number_with_format(
                row, col,
                format_number(vendor_info.5), &format_bold
            ).unwrap();
        col -= 5;
        row += 1;
    });

    (row, col)
}