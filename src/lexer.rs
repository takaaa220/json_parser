#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    String(String), // 文字列
    Number(f64),    // 数値
    Bool(bool),     // 真偽値
    Null,           // Null
    WhiteSpace,     // 空白
    LeftBrace,      // {
    RightBrace,     // }
    LeftBracket,    // [
    RightBracket,   // ]
    Comma,          // ,
    Colon,          // :
}

// JSONの文字列をParseして Token 単位に分割
pub struct Lexer<'a> {
    /// 読込中の先頭文字列を指す
    chars: std::iter::Peekable<std::str::Chars<'a>>,
}

/// 字句解析中に発生したエラー
#[derive(Debug)]
pub struct LexerError {
    /// エラーメッセージ
    pub msg: String,
}

impl LexerError {
    fn new(msg: &str) -> LexerError {
        LexerError {
            msg: msg.to_string(),
        }
    }
}

impl<'a> Lexer<'a> {
    /// 文字列を受け取り Lexer を渡す
    pub fn new(input: &str) -> Lexer {
        Lexer {
            chars: input.chars().peekable(),
        }
    }

    /// 文字列を Token 単位に分割する
    pub fn tokenize(&mut self) -> Result<Vec<Token>, LexerError> {
        let mut tokens = vec![];

        while let Some(token) = self.next_token()? {
            match token {
                // 空白は今回は捨てるがデバッグ情報として使える(行、列)
                Token::WhiteSpace => {}
                _ => {
                    tokens.push(token);
                }
            }
        }

        Ok(tokens)
    }

    /// 一文字分だけ読み進め、tokenを返す
    fn next_return_token(&mut self, token: Token) -> Option<Token> {
        self.chars.next();
        Some(token)
    }

    /// 文字列を読み込み、マッチしたTokenを返す
    fn next_token(&mut self) -> Result<Option<Token>, LexerError> {
        // 先頭の文字列を読み込む
        match self.chars.peek() {
            Some(c) => match c {
                c if c.is_whitespace() || *c == '\n' => {
                    Ok(self.next_return_token(Token::WhiteSpace))
                }
                '{' => Ok(self.next_return_token(Token::LeftBrace)),
                '}' => Ok(self.next_return_token(Token::LeftBrace)),
                '[' => Ok(self.next_return_token(Token::LeftBracket)),
                ']' => Ok(self.next_return_token(Token::RightBracket)),
                ',' => Ok(self.next_return_token(Token::Comma)),
                ':' => Ok(self.next_return_token(Token::Colon)),

                // NOTE
                // 以下のマッチ条件は開始文字が該当する Token の開始文字なら、Token の文字列分だけ読み進める

                // String は開始文字列 '"'
                // e.g. "togatoga"
                '"' => {
                    self.chars.next();
                    self.parse_string_token()
                }

                // Number は開始文字が[0-9] or ('+', '-', '.')
                // e.g. 1, -1235, +10, .001
                c if c.is_numeric() || matches!(c, '+' | '-' | '.') => self.parse_number_token(),

                // Boolean の true の開始文字は 't'
                't' => self.parse_bool_token(true),

                // Boolean の false の花医師文字は 'f'
                'f' => self.parse_bool_token(false),

                // Null の開始文字は 'n'
                'n' => self.parse_null_token(),

                // 上記のルールにマッチしない文字はエラー
                _ => Err(LexerError::new(&format!("error: an unexpected char {}", c))),
            },
            None => Ok(None),
        }
    }

    /// nullの文字列をparseする
    fn parse_null_token(&mut self) -> Result<Option<Token>, LexerError> {
        let s = (0..4).filter_map(|_| self.chars.next()).collect::<String>();

        if s == "null" {
            Ok(Some(Token::Null))
        } else {
            Err(LexerError::new(&format!(
                "error: a null value is expected {}",
                s
            )))
        }
    }

    /// (true|false)の文字列をparseする
    fn parse_bool_token(&mut self, b: bool) -> Result<Option<Token>, LexerError> {
        if b {
            let s = (0..4).filter_map(|_| self.chars.next()).collect::<String>();

            if s == "true" {
                Ok(Some(Token::Bool(true)))
            } else {
                Err(LexerError::new(&format!(
                    "error: a boolean true is expected {}",
                    s
                )))
            }
        } else {
            let s = (0..5).filter_map(|_| self.chars.next()).collect::<String>();

            if s == "false" {
                Ok(Some(Token::Bool(false)))
            } else {
                Err(LexerError::new(&format!(
                    "error: a boolean false is expected {}",
                    s
                )))
            }
        }
    }

    /// 数字として使用可能な文字まで読み込む。読み込んだ文字列が数字(`f64`)としてParseに成功した場合Tokenを返す。
    fn parse_number_token(&mut self) -> Result<Option<Token>, LexerError> {
        let mut number_str = String::new();

        while let Some(&c) = self.chars.peek() {
            // 数字に使われる可能性がある文字は読み込み、そうではない文字の場合は読み込みを終了する
            if c.is_numeric() | matches!(c, '+' | '-' | 'e' | 'E' | '.') {
                self.chars.next();
                number_str.push(c);
            } else {
                break;
            }
        }

        // 読み込んだ文字列がParseできた場合はTokenを返す
        match number_str.parse::<f64>() {
            Ok(number) => Ok(Some(Token::Number(number))),
            Err(e) => Err(LexerError::new(&format!("error: {}", e.to_string()))),
        }
    }

    /// 終端文字'\"'まで文字列を読み込む。UTF-16(\u0000~\uFFFF)や特殊なエスケープ文字(e.g. '\t','\n')も考慮する
    fn parse_string_token(&mut self) -> Result<Option<Token>, LexerError> {
        let mut utf16: Vec<u16> = vec![];
        let mut result = String::new();

        while let Some(c1) = self.chars.next() {
            match c1 {
                // Escapeの開始文字
                '\\' => {
                    let c2 = self
                        .chars
                        .next()
                        .ok_or_else(|| LexerError::new("error: a next char is expected"))?;
                    if matches!(c2, '"' | '\\' | '/' | 'b' | 'f' | 'n' | 'r' | 't') {
                        // エスケープ文字列の処理
                        // https://www.rfc-editor.org/rfc/rfc8259#section-7
                        // utf-16のバッファを文字列にpushしておく
                        Self::push_utf16(&mut result, &mut utf16)?;
                        result.push('\\');
                        result.push(c2);
                    } else if c2 == 'u' {
                        // UTF-16
                        // \u0000 ~ \uFFFF
                        // \uまで読み込んだので残りの0000~XXXXの4文字を読み込む
                        // UTF-16に関してはエスケープ処理を行う
                        let hexs = (0..4)
                            .filter_map(|_| {
                                let c = self.chars.next()?;
                                if c.is_ascii_hexdigit() {
                                    Some(c)
                                } else {
                                    None
                                }
                            })
                            .collect::<Vec<_>>();

                        match u16::from_str_radix(&hexs.iter().collect::<String>(), 16) {
                            Ok(code_point) => utf16.push(code_point),
                            Err(e) => {
                                return Err(LexerError::new(&format!(
                                    "error: a unicode character is expected {}",
                                    e.to_string()
                                )))
                            }
                        };
                    } else {
                        return Err(LexerError::new(&format!(
                            "error: an unexpected escaped char {}",
                            c2
                        )));
                    }
                }
                // 文字列の終端
                '\"' => {
                    Self::push_utf16(&mut result, &mut utf16)?;
                    return Ok(Some(Token::String(result)));
                }
                // それ以外の文字列
                _ => {
                    Self::push_utf16(&mut result, &mut utf16)?;
                    result.push(c1);
                }
            }
        }

        // 文字列の終端である '"' が存在しない場合はエラー
        Err(LexerError::new(&"error: not close string"))
    }

    /// utf16のバッファが存在するならば連結しておく
    fn push_utf16(result: &mut String, utf16: &mut Vec<u16>) -> Result<(), LexerError> {
        if utf16.is_empty() {
            return Ok(());
        }
        match String::from_utf16(utf16) {
            Ok(utf16_str) => {
                result.push_str(&utf16_str);
                utf16.clear();
            }
            Err(e) => {
                return Err(LexerError::new(&format!("error: {}", e.to_string())));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_null() {
        let null = "null";
        let tokens = Lexer::new(null).tokenize().unwrap();
        assert_eq!(tokens[0], Token::Null);
    }

    #[test]
    fn test_bool() {
        let false_str: &str = "false";
        let tokens = Lexer::new(false_str).tokenize().unwrap();
        assert_eq!(tokens[0], Token::Bool(false));

        let true_str: &str = "true";
        let tokens = Lexer::new(true_str).tokenize().unwrap();
        assert_eq!(tokens[0], Token::Bool(true));
    }

    #[test]
    fn test_number() {
        let number_strs = [
            ("3", Token::Number(3.0)),
            ("+3", Token::Number(3.0)),
            ("-3", Token::Number(-3.0)),
            ("1e3", Token::Number(1000.0)),
            ("0.3", Token::Number(0.3)),
            (".3", Token::Number(0.3)),
        ];
        number_strs.map(|(input, expect)| {
            let tokens = Lexer::new(input).tokenize().unwrap();
            assert_eq!(tokens[0], expect);
        });

        let tokens = Lexer::new("+-3").tokenize();
        assert!(tokens.is_err());
    }

    #[test]
    fn test_string() {
        let string_strs = [
            ("\"hello world\"", Token::String("hello world".to_string())),
            ("\"あいうえお\"", Token::String("あいうえお".to_string())),
            (
                r#""\u3042\u3044\u3046abc""#,
                Token::String("あいうabc".to_string()),
            ),
            (
                r#""\uD83D\uDE04\uD83D\uDE07\uD83D\uDC7A""#,
                Token::String(r#"😄😇👺"#.to_string()),
            ),
        ];
        string_strs.map(|(input, expect)| {
            let tokens = Lexer::new(input).tokenize().unwrap();
            assert_eq!(tokens[0], expect);
        });

        let tokens = Lexer::new("\"hello world").tokenize();
        assert!(tokens.is_err());
    }
}
