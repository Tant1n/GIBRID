// NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.
// (Это неофициальный продукт Minecraft, не одобрен Mojang или Microsoft и не связан с nimi.

//! Зашифрованное TCP-соединение с сервером Minecraft.
//! Протокол Minecraft использует AES-128/CFB8 после обмена ключами (Encryption Request/Response).
//! Реализация CFB8 на основе блочного шифра AES-128 из crates `aes` + `cipher`.
//! RSA-обмен shared secret не реализован (требует криптобэкенда), ключ устанавливается вручную
//! после завершения рукоплечания шифрования.

use aes::Aes128;
use aes::cipher::{Array, Block, BlockCipherEncrypt, KeyInit};
use std::io::{self, Read, Write};
use std::net::TcpStream;

/// Параметры сессионного шифрования Minecraft.
#[derive(Debug, Clone)]
pub struct EncryptionKeys {
    /// 16-байтовый shared secret.
    pub key: [u8; 16],
    /// 16-байтовый IV.
    pub iv: [u8; 16],
}

/// TCP-поток с AES-128/CFB8 шифрованием.
pub struct EncryptedStream {
    stream: TcpStream,
    cipher: Aes128,
    iv_enc: [u8; 16],
    iv_dec: [u8; 16],
}

impl EncryptedStream {
    /// Создать зашифрованный поток из уже существующего TcpStream и ключей.
    pub fn new(stream: TcpStream, keys: &EncryptionKeys) -> io::Result<Self> {
        let cipher = Aes128::new_from_slice(&keys.key)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        Ok(Self {
            stream,
            cipher,
            iv_enc: keys.iv,
            iv_dec: keys.iv,
        })
    }

    /// Зашифровать один байт в режиме CFB8 и записать в поток.
    fn encrypt_byte(&mut self, byte: u8) -> io::Result<()> {
        let mut block: Block<Aes128> = Array::default();
        block.as_mut_slice().copy_from_slice(&self.iv_enc);
        self.cipher.encrypt_block(&mut block);
        let keystream_byte = block[0];
        let ciphertext = byte ^ keystream_byte;
        // Сдвигаем IV на 1 байт влево и ставит зашифрованный байт в конец
        for i in 0..15 {
            self.iv_enc[i] = self.iv_enc[i + 1];
        }
        self.iv_enc[15] = ciphertext;
        self.stream.write_all(&[ciphertext])?;
        Ok(())
    }

    /// Расшифровать один байт из потока.
    fn decrypt_byte(&mut self) -> io::Result<u8> {
        let mut block: Block<Aes128> = Array::default();
        block.as_mut_slice().copy_from_slice(&self.iv_dec);
        self.cipher.encrypt_block(&mut block);
        let keystream_byte = block[0];
        let mut ciphertext_byte = [0u8; 1];
        self.stream.read_exact(&mut ciphertext_byte)?;
        let plaintext = ciphertext_byte[0] ^ keystream_byte;
        // Сдвигаем IV на 1 байт влево и ставит ciphertext байт в конец
        for i in 0..15 {
            self.iv_dec[i] = self.iv_dec[i + 1];
        }
        self.iv_dec[15] = ciphertext_byte[0];
        Ok(plaintext)
    }

    /// Записать зашифрованные данные (по байтно).
    pub fn write_encrypted(&mut self, buf: &[u8]) -> io::Result<usize> {
        for &byte in buf {
            self.encrypt_byte(byte)?;
        }
        Ok(buf.len())
    }

    /// Прочитать и расшифровать ровно `len` байт.
    pub fn read_exact_encrypted(&mut self, len: usize) -> io::Result<Vec<u8>> {
        let mut out = Vec::with_capacity(len);
        for _ in 0..len {
            out.push(self.decrypt_byte()?);
        }
        Ok(out)
    }

    /// Получить доступ к исходному потоку (для нешифрованных операций до установки ключа).
    pub fn inner_stream(&mut self) -> &mut TcpStream {
        &mut self.stream
    }
}

impl Read for EncryptedStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        for byte in buf.iter_mut() {
            *byte = self.decrypt_byte()?;
        }
        Ok(buf.len())
    }
}

impl Write for EncryptedStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.write_encrypted(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

/// Генерация случайного shared secret (16 байт) для сессии шифрования.
pub fn generate_shared_secret() -> [u8; 16] {
    let mut buf = [0u8; 16];
    getrandom::fill(&mut buf).expect("getrandom failed");
    buf
}

/// Генерация случайного IV (16 байт).
pub fn generate_iv() -> [u8; 16] {
    let mut buf = [0u8; 16];
    getrandom::fill(&mut buf).expect("getrandom failed");
    buf
}
