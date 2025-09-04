
#import "@preview/shiroa:0.2.0": *

#show: book

#book-meta(
  language: "ja",
  title: "Typst Japanese Community",
  authors: ("Typst Japanese Community contributors",),
  description: "Typst日本語コミュニティのランディングページ",
  repository: "https://github.com/typst-jp/typst-jp.github.io",
  summary: [
    #prefix-chapter("index.typ")[Home]
  ],
)



// re-export page template
#import "/templates/page.typ": project
#let book-page = project
