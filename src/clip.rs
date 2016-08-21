use clipboard::ClipboardContext;

pub struct Clipboard {
    real_clipboard: Option<ClipboardContext>,
    contents: String,
}

impl Clipboard {
    pub fn new() -> Self {
        Clipboard {
            real_clipboard: ClipboardContext::new().ok(),
            contents: String::new(),
        }
    }

    pub fn set<S: AsRef<str>>(&mut self, s: S) {
        self.contents = s.as_ref().to_owned();
        if let Some(clip) = self.real_clipboard.as_mut() {
            // throw away result -- internal clipboard will fail over
            let _ = clip.set_contents(s.as_ref().to_owned());
        }
    }

    pub fn get(&self) -> String {
        let internal_contents = self.contents.clone();
        match self.real_clipboard.as_ref() {
            Some(clip) => clip.get_contents().unwrap_or(internal_contents),
            None => internal_contents,
        }
    }
}
