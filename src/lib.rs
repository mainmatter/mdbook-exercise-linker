use anyhow::{Context, Error};
use mdbook_preprocessor::book::{Book, BookItem};
use mdbook_preprocessor::{Preprocessor, PreprocessorContext};

pub struct ExerciseLinker;

impl ExerciseLinker {
    pub fn new() -> ExerciseLinker {
        ExerciseLinker
    }
}

impl Preprocessor for ExerciseLinker {
    fn name(&self) -> &str {
        "exercise-linker"
    }

    fn run(&self, ctx: &PreprocessorContext, mut book: Book) -> Result<Book, Error> {
        let root_url: String = ctx
            .config
            .get("preprocessor.exercise-linker.exercise_root_url")
            .context("Failed to get `exercise_root_url`")?
            .context("`exercise_root_url` is not set")?;

        book.items
            .iter_mut()
            .for_each(|i| process_book_item(i, &ctx.renderer, &root_url));
        Ok(book)
    }

    fn supports_renderer(&self, _renderer: &str) -> mdbook_preprocessor::errors::Result<bool> {
        Ok(true)
    }
}

fn process_book_item(item: &mut BookItem, renderer: &str, root_url: &str) {
    match item {
        BookItem::Chapter(chapter) => {
            chapter.sub_items.iter_mut().for_each(|item| {
                process_book_item(item, renderer, root_url);
            });

            let Some(source_path) = &chapter.source_path else {
                return;
            };
            let source_path = source_path.display().to_string();

            // Ignore non-exercise chapters
            if !source_path.chars().take(2).all(|c| c.is_digit(10)) {
                return;
            }

            // Ignore chapter intro pages, such as `02_basics/00_README.md`.
            // They have no exercise of their own, so a link would point at a
            // directory that does not exist.
            if is_intro_page(&source_path) {
                return;
            }

            let exercise_path = source_path.strip_suffix(".md").unwrap();
            let link = format!(
                "\nThe exercise for this section is located in [`{exercise_path}`]({}/{exercise_path})\n",
                root_url
            );

            // If the chapter already has an `## Exercise` section, insert the
            // link into it instead of adding a second, duplicate heading.
            if let Some(idx) = exercise_heading_end(&chapter.content) {
                chapter.content.insert_str(idx, &link);
            } else {
                chapter.content.push_str(&format!("\n## Exercise\n{link}"));
            }

            if renderer == "pandoc" {
                chapter.content.push_str("`\\newpage`{=latex}\n");
            }
        }
        BookItem::Separator => {}
        BookItem::PartTitle(_) => {}
    }
}

/// Returns whether `source_path` is a chapter intro page: a `README.md`, or a
/// file whose name ends in `_README.md`.
fn is_intro_page(source_path: &str) -> bool {
    let file_name = source_path.rsplit('/').next().unwrap_or(source_path);
    file_name == "README.md" || file_name.ends_with("_README.md")
}

/// Returns the byte offset just after an existing `## Exercise` heading line,
/// or `None` if the chapter has no such heading.
fn exercise_heading_end(content: &str) -> Option<usize> {
    let mut offset = 0;
    for line in content.split_inclusive('\n') {
        if line.trim_end() == "## Exercise" {
            return Some(offset + line.len());
        }
        offset += line.len();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use mdbook_preprocessor::book::Chapter;

    const ROOT: &str = "https://example.com/exercises";

    fn process(path: &str, content: &str) -> String {
        let mut item = BookItem::Chapter(Chapter::new("Test", content.to_string(), path, vec![]));
        process_book_item(&mut item, "html", ROOT);
        match item {
            BookItem::Chapter(chapter) => chapter.content,
            _ => unreachable!(),
        }
    }

    #[test]
    fn adds_an_exercise_section_to_a_section_page() {
        let content = process("02_basics/03_structs.md", "# Structs\n");
        assert_eq!(
            content,
            "# Structs\n\n## Exercise\n\nThe exercise for this section is located in \
             [`02_basics/03_structs`](https://example.com/exercises/02_basics/03_structs)\n"
        );
    }

    #[test]
    fn fills_an_existing_exercise_heading() {
        let content = process("02_basics/03_structs.md", "# Structs\n\n## Exercise\n");
        assert_eq!(content.matches("## Exercise").count(), 1);
        assert!(content.contains("(https://example.com/exercises/02_basics/03_structs)"));
    }

    #[test]
    fn skips_chapter_intro_pages() {
        for path in ["02_basics/00_README.md", "02_basics/README.md"] {
            let original = "# Chapter 2\n\n## Exercises\n\nSee `exercises/02_basics`.\n";
            assert_eq!(process(path, original), original, "{path}");
        }
    }

    #[test]
    fn keeps_intro_pages_that_have_an_exercise() {
        // 100 Exercises to Learn Rust has an exercise for `00_intro.md`.
        let content = process("02_basic_calculator/00_intro.md", "# Intro\n");
        assert!(content.contains("(https://example.com/exercises/02_basic_calculator/00_intro)"));
    }

    #[test]
    fn skips_pages_outside_numbered_chapters() {
        assert_eq!(process("welcome.md", "# Welcome\n"), "# Welcome\n");
    }
}
