use std::path::Path;

pub fn get_mime_tipe(path: &String) -> String {
    let extension = Path::new(path.as_str())
        .extension()
        .unwrap()
        .to_str()
        .unwrap();
    let mimetipe = match extension {
        "js" => "text/javascript",
        "json" => "application/json",
        "css" => "text/css",
        "html" => "text/html",
        "ico" => "image/x-icon",
        _ => "text/plain",
    };

    mimetipe.to_string()
}

//TODO: make a parser args to config
//pub fn args_to_dict(list: Vec<String>) -> HashMap<String, String> {}
