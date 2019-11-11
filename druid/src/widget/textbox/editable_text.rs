use crate::unicode_segmentation::GraphemeCursor;

pub trait EditableText {
    // pub fn edit<T: IntervalBounds>(&mut self, iv: T, new: String);

    // pub fn slice<T IV>(&self, iv: T) -> String;

    // pub fn is_codepoint_boundary(&self, offset: usize) -> bool;

    // pub fn prev_codepoint_offset(&self, offset: usize) -> Option<usize>;

    // pub fn next_codepoint_offset(&self, offset: usize) -> Option<usize>;

    // pub fn at_or_next_codepoint_boundary(&self, offset: usize) -> Option<usize>;

    // pub fn at_or_prev_codepoint_boundary(&self, offset: usize) -> Option<usize>;

    fn prev_grapheme_offset(&self, offset: usize) -> Option<usize>;

    fn next_grapheme_offset(&self, offset: usize) -> Option<usize>;
}

impl EditableText for String {
    /// Gets the next grapheme from the given index.
    fn next_grapheme_offset(&self, from: usize) -> Option<usize> {
        let mut c = GraphemeCursor::new(from, self.len(), true);
        let next_boundary = c.next_boundary(self, 0).unwrap();
        next_boundary
    }

    /// Gets the previous grapheme from the given index.
    fn prev_grapheme_offset(&self, from: usize) -> Option<usize> {
        let mut c = GraphemeCursor::new(from, self.len(), true);
        let prev_boundary = c.prev_boundary(self, 0).unwrap();
        prev_boundary
    }
}

// pub trait EditableTextCursor {
// 	pub fn new(s: &'a String, position: usize) -> EditableTextCursor;

// 	pub fn total_len(&self) -> usize;

// 	pub fn text(&self) -> &'a String;

// 	// set cursor position
// 	pub fn set(&mut self, position: usize);

// 	// get cursor position
//     pub fn pos(&self) -> usize;

//     pub fn is_boundary(&self) -> bool;

// 	// moves cursor to previous boundary if exists
// 	// else becomes invalid cursor
// 	pub fn prev(&mut self) -> Option<(usize)>;

// 	pub fn next(&mut self) -> Option<(usize)>;

// 	//return current if it's a boundary, else next
// 	pub fn at_or_next(&mut self) -> Option<usize>;

// 	pub fn at_or_prev(&mut self) -> Option<usize>;

// 	// pub fn iter(&mut self) -> CursorIter???
// }

// struct StringCursor {
// 	text: &'a String,
// 	position: usize
// }

// impl<'a> EditableTextCursor for StringCursor<'a> {
//     pub fn new(text: &'a String, position: usize) -> StringCursor {
//         StringCursor { text, position }
//     }

//     pub fn total_len(&self) -> usize {
//         self.text.len()
//     }

//     pub fn text(&self) -> &'a String {
//         self.text
//     }

// 	// set cursor position
// 	pub fn set(&mut self, position: usize) {
//         self.position = position;
//     }

// 	// get cursor position
// 	pub fn pos(&self) -> usize {
//         self.position
//     }

//     pub fn is_boundary(&mut self) -> bool {
//         self.text.is_char_boundary(self.position)
//     }

// 	// moves cursor to previous boundary if exists
// 	// else becomes invalid cursor
// 	pub fn prev(&mut self) -> Option<(usize)> {
//         if self.position == 0 {
//             return None;
//         }

//         // This seems wasteful but I don't have a "chunk" concept to help out
//         let mut iter = self.text[..self.position].char_indices().rev().enumerate();
//         let (mut i, mut ch) = iter.next().unwrap();
//         loop {
//             if self.text.is_char_boundary(i) {
//                 return Some(i)
//             } else {
//                 // PROBABLY WENT WRONG HERE
//                 (i, ch) = iter.next().unwrap();
//             }
//         }

//     }

// 	pub fn next(&mut self) -> Option<(usize)>;

// 	//return current if it's a boundary, else next
// 	pub fn at_or_next(&mut self) -> Option<usize>;

// 	pub fn at_or_prev(&mut self) -> Option<usize>;
// }
