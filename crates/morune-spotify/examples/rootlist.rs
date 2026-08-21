//! Le o rootlist gravado pela sonda e mostra o que o Morune extrai dele.
//!
//! Serve para conferir a leitura contra a resposta real de uma conta sem
//! precisar de login. O arquivo vem de `cargo run --example sonda`.
//!
//! ```powershell
//! cargo run --release --example rootlist -p morune-spotify
//! ```

fn main() {
    let caminho = "bench-out/sonda/rootlist.protobuf";
    let bytes = match std::fs::read(caminho) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("{caminho}: {e} -- rode a sonda antes");
            std::process::exit(1);
        }
    };

    let resumos = morune_spotify::debug_rootlist(&bytes).expect("rootlist ilegivel");
    println!(
        "{} playlists lidas de {} bytes\n",
        resumos.len(),
        bytes.len()
    );

    for (nome, dono, tamanho, formato) in resumos.iter().take(40) {
        let marca = if !formato.is_empty() || dono.eq_ignore_ascii_case("spotify") {
            "[spotify]"
        } else {
            "[   sua ]"
        };
        println!("  {marca} {nome:<44.44} {tamanho:>5} faixas  dono={dono}");
    }
}
