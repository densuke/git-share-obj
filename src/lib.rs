//! Gitオブジェクトの重複ファイルをハードリンクで共有するライブラリ

pub mod cli;
pub mod fsck;
pub mod hardlink;
pub mod i18n;
pub mod scanner;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert!(true);
    }
}
