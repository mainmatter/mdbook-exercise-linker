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
