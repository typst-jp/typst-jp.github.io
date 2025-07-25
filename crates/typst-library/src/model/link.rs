use std::ops::Deref;

use ecow::{eco_format, EcoString};

use crate::diag::{bail, warning, At, SourceResult, StrResult};
use crate::engine::Engine;
use crate::foundations::{
    cast, elem, Content, Label, NativeElement, Packed, Repr, Show, ShowSet, Smart,
    StyleChain, Styles, TargetElem,
};
use crate::html::{attr, tag, HtmlElem};
use crate::introspection::Location;
use crate::layout::Position;
use crate::text::TextElem;

/// URLや文書中の位置へのリンク。
///
/// デフォルトでは、リンクの外見は通常のテキストと変わりません。
/// しかし、showルールを使うことで、簡単に任意のスタイルを適用できます。
///
/// # 例
/// ```example
/// #show link: underline
///
/// https://example.com \
///
/// #link("https://example.com") \
/// #link("https://example.com")[
///   See example.com
/// ]
/// ```
///
/// # ハイフネーション
/// ハイフネーションや両端揃えを有効にしていても、意図しないURL中のハイフネーションを防ぐため、
/// デフォルトではリンクには適用されません。
/// これを無効化するには、`{show link: set text(hyphenate: true)}`を使用します。
///
/// # 構文
/// この関数には専用の構文もあります。
/// `http://`や`https://`で始まるテキストは、自動的にリンクに変換されます。
#[elem(Show)]
pub struct LinkElem {
    /// リンクの遷移先。
    ///
    /// - Webページにリンクする場合、`dest`は有効なURL文字列である必要があります。
    ///   `mailto:`や`tel:`のURLスキームを含むURLが指定され、
    ///   かつ`body`パラメーターが省略された場合、
    ///   URLスキームを除いたメールアドレスまたは電話番号がリンクの本文になります。
    ///
    /// - 文書中の別の部分にリンクする場合、
    ///     `dest`には次の3つのうちいずれかの形式を用いることができます。
    ///   - 要素に付与された[label]。
    ///     要素に基づいて自動的にリンクの本文を生成したい場合は、
    ///     [reference]($ref)の使用を検討してください。
    ///
    ///   - [`location`]（通常は[`here`]や[`locate`]、
    ///     [`query`]から取得される）。
    ///
    ///   - [integer]($int)型の`page`キーと[length]型の`x`座標、`y`座標を持つ辞書。
    ///     ページ番号は1から始まり、
    ///     座標はページの左上隅からの相対位置です。
    ///
    /// ```example
    /// = Introduction <intro>
    /// #link("mailto:hello@typst.app") \
    /// #link(<intro>)[Go to intro] \
    /// #link((page: 1, x: 0pt, y: 0pt))[
    ///   Go to top
    /// ]
    /// ```
    #[required]
    #[parse(
        let dest = args.expect::<LinkTarget>("destination")?;
        dest.clone()
    )]
    pub dest: LinkTarget,

    /// リンクとして表示するコンテンツ。
    ///
    /// `dest`がURL文字列の場合、このパラメーターは省略可能です。
    /// この場合、URLがリンクとして表示されます。
    #[required]
    #[parse(match &dest {
        LinkTarget::Dest(Destination::Url(url)) => match args.eat()? {
            Some(body) => body,
            None => body_from_url(url),
        },
        _ => args.expect("body")?,
    })]
    pub body: Content,

    /// A destination style that should be applied to elements.
    #[internal]
    #[ghost]
    pub current: Option<Destination>,
}

impl LinkElem {
    /// Create a link element from a URL with its bare text.
    pub fn from_url(url: Url) -> Self {
        let body = body_from_url(&url);
        Self::new(LinkTarget::Dest(Destination::Url(url)), body)
    }
}

impl Show for Packed<LinkElem> {
    #[typst_macros::time(name = "link", span = self.span())]
    fn show(&self, engine: &mut Engine, styles: StyleChain) -> SourceResult<Content> {
        let body = self.body.clone();

        Ok(if TargetElem::target_in(styles).is_html() {
            if let LinkTarget::Dest(Destination::Url(url)) = &self.dest {
                HtmlElem::new(tag::a)
                    .with_attr(attr::href, url.clone().into_inner())
                    .with_body(Some(body))
                    .pack()
                    .spanned(self.span())
            } else {
                engine.sink.warn(warning!(
                    self.span(),
                    "non-URL links are not yet supported by HTML export"
                ));
                body
            }
        } else {
            match &self.dest {
                LinkTarget::Dest(dest) => body.linked(dest.clone()),
                LinkTarget::Label(label) => {
                    let elem = engine.introspector.query_label(*label).at(self.span())?;
                    let dest = Destination::Location(elem.location().unwrap());
                    body.clone().linked(dest)
                }
            }
        })
    }
}

impl ShowSet for Packed<LinkElem> {
    fn show_set(&self, _: StyleChain) -> Styles {
        let mut out = Styles::new();
        out.set(TextElem::set_hyphenate(Smart::Custom(false)));
        out
    }
}

fn body_from_url(url: &Url) -> Content {
    let text = ["mailto:", "tel:"]
        .into_iter()
        .find_map(|prefix| url.strip_prefix(prefix))
        .unwrap_or(url);
    let shorter = text.len() < url.len();
    TextElem::packed(if shorter { text.into() } else { (**url).clone() })
}

/// A target where a link can go.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum LinkTarget {
    Dest(Destination),
    Label(Label),
}

cast! {
    LinkTarget,
    self => match self {
        Self::Dest(v) => v.into_value(),
        Self::Label(v) => v.into_value(),
    },
    v: Destination => Self::Dest(v),
    v: Label => Self::Label(v),
}

impl From<Destination> for LinkTarget {
    fn from(dest: Destination) -> Self {
        Self::Dest(dest)
    }
}

/// A link destination.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Destination {
    /// A link to a URL.
    Url(Url),
    /// A link to a point on a page.
    Position(Position),
    /// An unresolved link to a location in the document.
    Location(Location),
}

impl Destination {}

impl Repr for Destination {
    fn repr(&self) -> EcoString {
        eco_format!("{self:?}")
    }
}

cast! {
    Destination,
    self => match self {
        Self::Url(v) => v.into_value(),
        Self::Position(v) => v.into_value(),
        Self::Location(v) => v.into_value(),
    },
    v: Url => Self::Url(v),
    v: Position => Self::Position(v),
    v: Location => Self::Location(v),
}

/// A uniform resource locator with a maximum length.
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Url(EcoString);

impl Url {
    /// Create a URL from a string, checking the maximum length.
    pub fn new(url: impl Into<EcoString>) -> StrResult<Self> {
        let url = url.into();
        if url.len() > 8000 {
            bail!("URL is too long")
        }
        Ok(Self(url))
    }

    /// Extract the underlying [`EcoString`].
    pub fn into_inner(self) -> EcoString {
        self.0
    }
}

impl Deref for Url {
    type Target = EcoString;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

cast! {
    Url,
    self => self.0.into_value(),
    v: EcoString => Self::new(v)?,
}
