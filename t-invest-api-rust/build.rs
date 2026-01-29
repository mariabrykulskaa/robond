use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let contracts_dir = "invest-contracts/src/docs/contracts";

    // Автоматически находим все .proto файлы в директории
    let mut proto_files = Vec::new();
    let entries = fs::read_dir(contracts_dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        // Добавляем только файлы (не директории) с расширением .proto
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("proto") {
            proto_files.push(path.to_str().unwrap().to_string());
        }
    }

    // Сортируем для детерминированного порядка
    proto_files.sort();

    // Компилируем все proto файлы одновременно в один модуль
    tonic_prost_build::configure()
        .build_server(false)
        .compile_protos(&proto_files, &[contracts_dir.to_string()])?;

    Ok(())
}
