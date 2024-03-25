use gesist::{decode_from_base64, encode_to_base64};

fn main() {
    while let Ok(input) = dialoguer::Input::<String>::new().with_prompt("Encoding to base64").interact_text() {
        let s = encode_to_base64(input.as_bytes());
        println!("Encoded: {}", s);
        println!("Decoded: {}", String::from_utf8(decode_from_base64(s.as_bytes()).unwrap().unwrap().into()).unwrap())
    }

    return;
}
