# singletonzip

Treat your zip file as normal file.

---

## Example

```rust
use singletonzip::{ Writer, Reader };

fn main() {
    // write
    let mut w = Writer::create(Path::new("mytext.txt.zip")).unwarp();
    w.write_all("singletonzip".as_bytes()).unwarp();
    w.finish().unwarp();

    // read
    let mut r = Reader::open("mytext.txt.zip").unwarp();
    let mut s = String::new();
    r.read_to_string(&mut s).unwarp();

    assert!(s.eq("singletonzip"));
}
```
