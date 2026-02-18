//! Gitオブジェクトの重複ファイルをハードリンクで共有するライブラリ

pub mod cli;
pub mod scanner;
pub mod hardlink;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert!(true);
    }
}
