
#import "@preview/shiroa:0.2.0": *

#show: book

#book-meta(
  title: "Typst Japanese Community",
  summary: [
    #prefix-chapter("index.typ")[Home]
  ],
)



// re-export page template
#import "/templates/page.typ": project
#let book-page = project
