use base64::Engine;
use sha1::{Digest, Sha1};

fn vichan_salt(input: &str) -> String {
    let mut padded = input.as_bytes().to_vec();
    padded.extend_from_slice(b"H..");
    let mut salt = [padded[1], padded[2]];
    for byte in &mut salt {
        if !matches!(*byte, b'.'..=b'z') {
            *byte = b'.';
        }
        *byte = match *byte {
            b':' => b'A',
            b';' => b'B',
            b'<' => b'C',
            b'=' => b'D',
            b'>' => b'E',
            b'?' => b'F',
            b'@' => b'G',
            b'[' => b'a',
            b'\\' => b'b',
            b']' => b'c',
            b'^' => b'd',
            b'_' => b'e',
            b'`' => b'f',
            other => other,
        };
    }
    String::from_utf8(salt.to_vec()).unwrap()
}

pub fn tripcode(input: &str) -> Result<String, crate::error::RelayError> {
    let salt = vichan_salt(input);
    let hash = pwhash::unix::crypt(input, &salt)
        .map_err(|e| crate::error::RelayError::SponsorBuild(format!("tripcode crypt: {e:?}")))?;
    let s = hash.as_str();
    Ok(s[s.len().saturating_sub(10)..].to_string())
}

pub fn secure_tripcode(input: &str, key: &str) -> Result<String, crate::error::RelayError> {
    let (encoded, _, had_errors) = encoding_rs::SHIFT_JIS.encode(input);
    if had_errors {
        return Err(crate::error::RelayError::SponsorBuild(
            "secure tripcode: shift_jis encoding failed".into(),
        ));
    }
    let key_bytes = encoded.as_ref();
    let mut hasher = Sha1::new();
    hasher.update(key_bytes);
    hasher.update(key.as_bytes());
    let digest = hasher.finalize();
    let encoded_digest = base64::engine::general_purpose::STANDARD.encode(digest);
    let salt_part = encoded_digest
        .get(..4)
        .ok_or_else(|| {
            crate::error::RelayError::SponsorBuild("secure tripcode: bad digest".into())
        })?
        .replace('+', ".");
    let setting = format!("_..A.{salt_part}");
    let hash = pwhash::unix::crypt(key_bytes, &setting)
        .map_err(|e| crate::error::RelayError::SponsorBuild(format!("secure tripcode: {e:?}")))?;
    let s = hash.as_str();
    Ok(s[s.len().saturating_sub(10)..].to_string())
}
