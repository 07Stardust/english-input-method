//! 有界离线导入 CSV/TSV/XLSX；只解析值，不执行工作簿公式、宏或外链。

use std::collections::HashSet;
use std::io::{self, Cursor, Read};
use std::path::Path;

use calamine::{Reader, Xlsx};

mod columns;
mod preview;
use crate::study::Entry;
pub use columns::ImportColumns;
pub use preview::ImportPreview;

const MAX_FILE: u64 = 20 * 1024 * 1024;
const MAX_EXPANDED: u64 = 100 * 1024 * 1024;
const MAX_ROWS: usize = 50_000;
const MAX_CELL: usize = 4096;

pub fn read_table(path: &Path) -> io::Result<Vec<Vec<String>>> {
    if std::fs::metadata(path)?.len() > MAX_FILE {
        return Err(io::Error::other("词书文件超过 20 MiB"));
    }
    let bytes = std::fs::read(path)?;
    if bytes.len() as u64 > MAX_FILE {
        return Err(io::Error::other("词书文件过大"));
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let rows = match extension.as_str() {
        "xlsx" => {
            let mut archive =
                zip::ZipArchive::new(Cursor::new(&bytes)).map_err(io::Error::other)?;
            if archive.len() > 4096 {
                return Err(io::Error::other("工作簿包含过多压缩项"));
            }
            let mut expanded = 0u64;
            for index in 0..archive.len() {
                let mut file = archive.by_index(index).map_err(io::Error::other)?;
                expanded = expanded
                    .checked_add(file.size())
                    .ok_or_else(|| io::Error::other("工作簿过大"))?;
                if expanded > MAX_EXPANDED
                    || file.name().contains("vbaProject")
                    || file.name().contains("externalLinks/")
                {
                    return Err(io::Error::other("工作簿包含宏、外链或解压体积超过 100 MiB"));
                }
                let mut inflated = Vec::new();
                (&mut file)
                    .take(MAX_EXPANDED + 1)
                    .read_to_end(&mut inflated)?;
                if inflated.len() as u64 != file.size() {
                    return Err(io::Error::other("工作簿压缩项大小无效"));
                }
                if file.name().ends_with(".xml") || file.name().ends_with(".rels") {
                    validate_xml(&inflated, file.name().starts_with("xl/worksheets/"))?;
                }
            }
            let mut workbook = Xlsx::new(Cursor::new(&bytes)).map_err(io::Error::other)?;
            let range = workbook
                .worksheet_range_at(0)
                .ok_or_else(|| io::Error::other("工作簿没有工作表"))?
                .map_err(io::Error::other)?;
            if range.height() > MAX_ROWS + 1 || range.width() > 32 {
                return Err(io::Error::other("工作表超出行列限制"));
            }
            range
                .rows()
                .map(|row| row.iter().map(ToString::to_string).collect())
                .collect::<Vec<Vec<String>>>()
        }
        "csv" | "tsv" => {
            let (encoding, offset) =
                encoding_rs::Encoding::for_bom(&bytes).unwrap_or((encoding_rs::UTF_8, 0));
            let (decoded, _, errors) = encoding.decode(&bytes[offset..]);
            let text = if errors && offset == 0 {
                let (text, _, invalid) = encoding_rs::GBK.decode(&bytes);
                if invalid {
                    return Err(io::Error::other("文件编码无效"));
                }
                text.into_owned()
            } else if errors {
                return Err(io::Error::other("文件编码无效"));
            } else {
                decoded.into_owned()
            };
            let mut reader = csv::ReaderBuilder::new()
                .delimiter(if extension == "tsv" { b'\t' } else { b',' })
                .has_headers(false)
                .flexible(true)
                .from_reader(text.as_bytes());
            let mut rows = Vec::new();
            for record in reader.records() {
                if rows.len() > MAX_ROWS {
                    return Err(io::Error::other("词书超过 50,000 行"));
                }
                rows.push(
                    record
                        .map_err(io::Error::other)?
                        .iter()
                        .map(str::to_owned)
                        .collect(),
                );
            }
            rows
        }
        _ => return Err(io::Error::other("仅支持 CSV、TSV 和 XLSX")),
    };
    if rows
        .iter()
        .any(|row| row.len() > 32 || row.iter().any(|cell| cell.len() > MAX_CELL))
    {
        return Err(io::Error::other("单元格或列数超过限制"));
    }
    Ok(rows)
}

/// 在 Calamine 分配单元格范围前限制稀疏坐标，并拒绝公式和外部关系。
fn validate_xml(bytes: &[u8], worksheet: bool) -> io::Result<()> {
    use quick_xml::events::Event;
    let mut reader = quick_xml::Reader::from_reader(bytes);
    let mut cells = 0usize;
    loop {
        match reader.read_event().map_err(io::Error::other)? {
            Event::Start(element) | Event::Empty(element) => {
                let local = element.local_name();
                if worksheet && local.as_ref() == b"f" {
                    return Err(io::Error::other("请将公式转为值后导入"));
                }
                if worksheet && local.as_ref() == b"c" {
                    cells += 1;
                    if cells > (MAX_ROWS + 1) * 32 {
                        return Err(io::Error::other("工作表单元格过多"));
                    }
                }
                for attribute in element.attributes() {
                    let attribute = attribute.map_err(io::Error::other)?;
                    if attribute.key.as_ref() == b"TargetMode"
                        && attribute.value.as_ref() == b"External"
                    {
                        return Err(io::Error::other("工作簿含外部链接"));
                    }
                    if worksheet
                        && ((local.as_ref() == b"c" && attribute.key.as_ref() == b"r")
                            || (local.as_ref() == b"dimension" && attribute.key.as_ref() == b"ref"))
                    {
                        let text =
                            std::str::from_utf8(&attribute.value).map_err(io::Error::other)?;
                        for coordinate in text.split(':') {
                            validate_coordinate(coordinate)?;
                        }
                    }
                }
            }
            Event::DocType(_) => return Err(io::Error::other("工作簿含不支持的 XML 文档类型")),
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(())
}

fn validate_coordinate(text: &str) -> io::Result<()> {
    let mut column = 0usize;
    let mut row = 0usize;
    let mut digits = false;
    for byte in text.bytes() {
        if byte.is_ascii_uppercase() && !digits {
            column = column
                .checked_mul(26)
                .and_then(|n| n.checked_add((byte - b'A' + 1) as usize))
                .ok_or_else(|| io::Error::other("单元格坐标过大"))?;
        } else if byte.is_ascii_digit() {
            digits = true;
            row = row
                .checked_mul(10)
                .and_then(|n| n.checked_add((byte - b'0') as usize))
                .ok_or_else(|| io::Error::other("单元格坐标过大"))?;
        } else {
            return Err(io::Error::other("单元格坐标无效"));
        }
    }
    if column == 0 || column > 32 || row == 0 || row > MAX_ROWS + 1 {
        return Err(io::Error::other("工作表超出行列限制"));
    }
    Ok(())
}

pub fn preview(rows: &[Vec<String>], columns: &ImportColumns) -> ImportPreview {
    let mut result = ImportPreview::default();
    let mut seen = HashSet::new();
    for (index, row) in rows.iter().enumerate().skip(usize::from(columns.header)) {
        if row.iter().all(|cell| cell.trim().is_empty()) {
            continue;
        }
        let cell = |column: Option<usize>| {
            column
                .and_then(|index| row.get(index))
                .map(|value| value.trim().to_owned())
                .unwrap_or_default()
        };
        let list = |column| {
            cell(column)
                .split([';', '；', '|'])
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .collect()
        };
        let english = cell(Some(columns.english));
        if english.is_empty() || english.len() > 256 || english.chars().any(char::is_control) {
            result
                .errors
                .push(format!("第 {} 行：英文词条为空或无效", index + 1));
            continue;
        }
        if !seen.insert(english.to_lowercase()) {
            result.duplicates += 1;
            continue;
        }
        result.entries.push(Entry {
            english,
            meaning: cell(columns.meaning),
            part_of_speech: cell(columns.part_of_speech),
            example: cell(columns.example),
            level: cell(columns.level),
            tags: list(columns.tags),
            input_keys: list(columns.input_keys),
            equivalents: list(columns.equivalents),
        });
    }
    result
}

#[cfg(test)]
mod security_tests {
    #[test]
    fn rejects_sparse_allocation_formula_and_external_link() {
        for xml in [
            "<worksheet><c r='XFD1048576'/></worksheet>",
            "<worksheet><c r='A1'><f>1+1</f></c></worksheet>",
            "<Relationships><Relationship TargetMode='External'/></Relationships>",
            "<!DOCTYPE worksheet><worksheet/>",
        ] {
            assert!(super::validate_xml(xml.as_bytes(), true).is_err());
        }
        assert!(
            super::validate_xml(
                b"<worksheet><dimension ref='A1:H200'/><c r='H200'/></worksheet>",
                true
            )
            .is_ok()
        );
    }
}
