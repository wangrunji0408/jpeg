# JPEG Decoder

A toy JPEG decoder following [跟我寫 JPEG 解碼器 (Write a JPEG decoder with me)](https://github.com/MROS/jpeg_tutorial).

## Usage

```
cargo run --release -- input.jpg -o output.ppm
```

## Performance

| env             | this  | djpeg |
| --------------- | ----- | ----- |
| AMD 3700X, WSL2 | 0.19s | 0.17s |
| M1 Pro, macOS   | 0.14s | 0.07s |
