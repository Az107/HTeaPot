use std::{thread, time::Duration};

use hteapot::{Hteapot, HttpRequest, StreamedResponse};

fn main() {
    let server = Hteapot::new("localhost", 8081);
    server.listen(move |_req: HttpRequest| {
        let times = 5;
        StreamedResponse::new(move |sender| {
            for i in 0..times {
                let data = format!("{i}-abcd\n").into_bytes();
                let _ = sender.send(data);
                thread::sleep(Duration::from_secs(1));
            }
        })
    });
}
