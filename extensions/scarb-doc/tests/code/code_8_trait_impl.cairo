pub trait Summary<T> {
fn summarize(self: @T) -> ByteArray;
}

#[derive(Drop)]
pub struct NewsArticle {
    pub headline: ByteArray,
    pub location: ByteArray,
    pub author: ByteArray,
    pub content: ByteArray,
}

impl NewsArticleSummary of Summary<NewsArticle> {
    fn summarize(self: @NewsArticle) -> ByteArray {
        format!("{} by {} ({})", self.headline, self.author, self.location)
    }
}

#[derive(Drop)]
pub struct Tweet {
    pub username: ByteArray,
    pub content: ByteArray,
    pub reply: bool,
    pub retweet: bool,
}

impl TweetSummary of Summary<Tweet> {
    fn summarize(self: @Tweet) -> ByteArray {
        format!("{}: {}", self.username, self.content)
    }
}

trait MultipleGenericArgs<T, U> {}
