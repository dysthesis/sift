use std::marker::PhantomData;

use url::Url;

use crate::parser::Parser;

type Byte = u8;

pub trait ContentState {}
pub struct Content<S>
where
    S: ContentState,
{
    url: Url,
    raw_bytes: Option<Vec<Byte>>,
    parser: Option<Box<dyn Parser>>,
    _state: PhantomData<S>,
}

impl<S> Content<S>
where
    S: ContentState,
{
    pub fn new(url: Url) -> Content<Unfetched> {
        Content {
            url,
            raw_bytes: None,
            parser: None,
            _state: PhantomData::<Unfetched>,
        }
    }
}

pub struct Unfetched;
impl ContentState for Unfetched {}
impl Content<Unfetched> {
    pub fn fetch(&self) -> Content<Raw> {
        todo!("Implement fetching raw bytes")
    }
}

pub struct Raw;
impl ContentState for Raw {}
impl Content<Raw> {
    pub fn identify(&self) {
        todo!("Implement identifying content type")
    }
}

pub struct Identified;
impl ContentState for Identified {}
impl Content<Identified> {
    pub fn parse(&self) {
        todo!("Implement parsing")
    }
}
