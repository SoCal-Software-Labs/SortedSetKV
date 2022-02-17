pub mod atoms;

use {
    log::error, rustler::Encoder, rustler::NifResult,
    sled::transaction::TransactionError as SledTransactionError, sled::IVec, sled::Transactional,
    std::convert::TryInto, std::io::Write,
};

const SCORE_PREFIX: &'static [u8; 6] = b"scores";
const KEY_PREFIX: &'static [u8; 4] = b"keys";
const LIST_PREFIX: &'static [u8; 5] = b"lists";
const VALUE_SUFFIX: &'static [u8; 1] = b"v";
const SCORE_SUFFIX: &'static [u8; 1] = b"s";

pub struct DbResource {
    pub db: sled::Db,
}

fn io_err_into(e: SledTransactionError<SledTransactionError>) -> rustler::error::Error {
    error!("Sled Error: {}", e.to_string());
    rustler::error::Error::Term(Box::new(atoms::sled_error()))
}

fn sled_err_into(e: sled::Error) -> rustler::error::Error {
    error!("Sled Error: {}", e.to_string());
    error!("Sled Error: {}", e.to_string());
    rustler::error::Error::Term(Box::new(atoms::sled_error()))
}

#[rustler::nif]
fn open<'a>(env: rustler::Env<'a>, a: String) -> NifResult<rustler::Term<'a>> {
    let config = sled::Config::default().path(&a);

    let db: sled::Db = config.open().map_err(sled_err_into)?;
    let db_resouce = rustler::ResourceArc::new(DbResource { db: db });
    Ok((atoms::ok(), db_resouce).encode(env))
}

#[rustler::nif]
fn clear<'a>(db_resouce: rustler::Term<'a>) -> NifResult<rustler::Atom> {
    let dbr: rustler::ResourceArc<DbResource> = db_resouce.decode()?;
    let db = &dbr.db;

    db.clear().map_err(sled_err_into)?;

    for name in db.tree_names() {
        if name
            != IVec::from(vec![
                95, 95, 115, 108, 101, 100, 95, 95, 100, 101, 102, 97, 117, 108, 116,
            ])
        {
            db.drop_tree(name).map_err(sled_err_into)?;
        }
    }

    Ok(atoms::ok())
}

#[rustler::nif]
fn zadd<'a>(
    db_resouce: rustler::Term<'a>,
    collection: rustler::Binary,
    key: rustler::Binary,
    value: Option<rustler::Binary>,
    score: Option<u64>,
    gt: bool,
) -> NifResult<rustler::Atom> {
    let dbr: rustler::ResourceArc<DbResource> = db_resouce.decode()?;
    let db = &dbr.db;
    let score_tree_bytes = SCORE_PREFIX
        .to_vec()
        .into_iter()
        .chain(collection.as_slice().iter().copied())
        .collect::<Vec<_>>();
    let key_tree_bytes = KEY_PREFIX
        .to_vec()
        .into_iter()
        .chain(collection.as_slice().iter().copied())
        .collect::<Vec<_>>();

    let score_tree: sled::Tree = db.open_tree(IVec::from(score_tree_bytes)).unwrap();
    let key_tree: sled::Tree = db.open_tree(IVec::from(key_tree_bytes)).unwrap();

    (&score_tree, &key_tree)
        .transaction(|(score_tree, key_tree)| {
            let score_bytes_key = score.map(|v| {
                v.to_be_bytes()
                    .to_vec()
                    .into_iter()
                    .chain(key.as_slice().iter().copied())
                    .collect::<Vec<_>>()
            });

            let value_key_bytes = key
                .as_slice()
                .to_vec()
                .into_iter()
                .chain(VALUE_SUFFIX.iter().copied())
                .collect::<Vec<_>>();

            let score_key_bytes = key
                .as_slice()
                .to_vec()
                .into_iter()
                .chain(SCORE_SUFFIX.iter().copied())
                .collect::<Vec<_>>();

            let kvvec = IVec::from(value_key_bytes);
            let ksvec = IVec::from(score_key_bytes);
            let mut insert = false;
            let mut old_value: Option<IVec> = None;

            match key_tree.get(ksvec.clone())? {
                Some(value) => {
                    old_value = Some(value.clone());
                    if gt {
                        let old_score: u64 = make_u64(&value);
                        if score.unwrap_or(u64::MAX) > old_score {
                            insert = true
                        }
                    } else {
                        insert = true
                    }
                }
                _ => insert = true,
            }

            if insert {
                if let Some(v) = value {
                    key_tree.insert(kvvec, v.as_slice())?;
                } else {
                    key_tree.remove(kvvec)?;
                }
                if let Some(s) = score {
                    let score_bin = s.to_be_bytes();
                    key_tree.insert(ksvec, &score_bin)?;
                } else {
                    key_tree.remove(ksvec)?;
                }
                if let Some(b) = score_bytes_key {
                    score_tree.insert(b, b"")?;
                }
                if let Some(old) = old_value {
                    let old_score_bytes = old
                        .to_vec()
                        .into_iter()
                        .chain(key.as_slice().iter().copied())
                        .collect::<Vec<_>>();
                    score_tree.remove(old_score_bytes)?;
                }
            }

            Ok(atoms::ok())
        })
        .map_err(io_err_into)
}

#[rustler::nif]
fn zscoreupdate<'a>(
    db_resouce: rustler::Term<'a>,
    collection: rustler::Binary,
    key: rustler::Binary,
    score: Option<u64>,
    gt: bool,
) -> NifResult<rustler::Atom> {
    let dbr: rustler::ResourceArc<DbResource> = db_resouce.decode()?;
    let db = &dbr.db;
    let score_tree_bytes = SCORE_PREFIX
        .to_vec()
        .into_iter()
        .chain(collection.as_slice().iter().copied())
        .collect::<Vec<_>>();
    let key_tree_bytes = KEY_PREFIX
        .to_vec()
        .into_iter()
        .chain(collection.as_slice().iter().copied())
        .collect::<Vec<_>>();

    let score_tree: sled::Tree = db.open_tree(IVec::from(score_tree_bytes)).unwrap();
    let key_tree: sled::Tree = db.open_tree(IVec::from(key_tree_bytes)).unwrap();

    (&score_tree, &key_tree)
        .transaction(|(score_tree, key_tree)| {
            let score_key_bytes = key
                .as_slice()
                .to_vec()
                .into_iter()
                .chain(SCORE_SUFFIX.iter().copied())
                .collect::<Vec<_>>();

            let ksvec = IVec::from(score_key_bytes);

            if let Some(value) = key_tree.get(ksvec.clone())? {
                let old_score: u64 = make_u64(&value);

                let old_score_bytes = old_score
                    .to_be_bytes()
                    .to_vec()
                    .into_iter()
                    .chain(key.as_slice().iter().copied())
                    .collect::<Vec<_>>();

                if let Some(s) = score {
                    let score_bytes = s
                        .to_be_bytes()
                        .to_vec()
                        .into_iter()
                        .chain(key.as_slice().iter().copied())
                        .collect::<Vec<_>>();

                    let should_commit = !gt || s > old_score;
                    if should_commit {
                        score_tree.remove(old_score_bytes)?;
                        score_tree.insert(score_bytes, b"")?;
                        key_tree.insert(ksvec, &s.to_be_bytes())?;
                    }
                } else {
                    key_tree.remove(ksvec)?;
                    score_tree.remove(old_score_bytes)?;
                }
            }

            Ok(atoms::ok())
        })
        .map_err(io_err_into)
}

#[rustler::nif]
fn zscore<'a>(
    db_resouce: rustler::Term<'a>,
    collection: rustler::Binary,
    key: rustler::Binary,
) -> NifResult<(bool, Option<u64>)> {
    let dbr: rustler::ResourceArc<DbResource> = db_resouce.decode()?;
    let db = &dbr.db;
    let key_tree_bytes = KEY_PREFIX
        .to_vec()
        .into_iter()
        .chain(collection.as_slice().iter().copied())
        .collect::<Vec<_>>();

    let key_tree: sled::Tree = db.open_tree(IVec::from(key_tree_bytes)).unwrap();
    let score_key_bytes = key
        .as_slice()
        .to_vec()
        .into_iter()
        .chain(SCORE_SUFFIX.iter().copied())
        .collect::<Vec<_>>();
    let value_key_bytes = key
        .as_slice()
        .to_vec()
        .into_iter()
        .chain(VALUE_SUFFIX.iter().copied())
        .collect::<Vec<_>>();

    let ksvec = IVec::from(score_key_bytes);
    let kvvec = IVec::from(value_key_bytes);

    match key_tree.get(ksvec.clone()).map_err(sled_err_into)? {
        Some(value) => {
            let score: u64 = make_u64(&value);
            Ok((true, Some(score)))
        }
        None => match key_tree.get(kvvec.clone()).map_err(sled_err_into)? {
            Some(_value) => Ok((true, None)),
            None => Ok((false, None)),
        },
    }
}

#[rustler::nif]
fn zrembyrangebyscore<'a>(
    db_resouce: rustler::Term<'a>,
    collection: rustler::Binary,
    min_score: u64,
    max_score: Option<u64>,
    limit: usize,
) -> NifResult<u64> {
    let dbr: rustler::ResourceArc<DbResource> = db_resouce.decode()?;
    let db = &dbr.db;
    let score_tree_bytes = SCORE_PREFIX
        .to_vec()
        .into_iter()
        .chain(collection.as_slice().iter().copied())
        .collect::<Vec<_>>();
    let key_tree_bytes = KEY_PREFIX
        .to_vec()
        .into_iter()
        .chain(collection.as_slice().iter().copied())
        .collect::<Vec<_>>();

    let score_tree: sled::Tree = db.open_tree(IVec::from(score_tree_bytes.clone())).unwrap();
    let key_tree: sled::Tree = db.open_tree(IVec::from(key_tree_bytes.clone())).unwrap();

    let min_bytes = min_score.to_be_bytes().to_vec();
    let score_byte_len = min_bytes.clone().len();

    let iter = if let Some(o) = max_score {
        score_tree
            .range(IVec::from(min_bytes)..IVec::from(o.to_be_bytes().to_vec()))
            .keys()
    } else {
        score_tree.range(IVec::from(min_bytes)..).keys()
    };

    let mut iter = iter.filter_map(|l| l.ok()).take(limit);

    let mut removed: u64 = 0;
    while let Some(k) = iter.next() {
        let score_key_bytes = k[score_byte_len..]
            .to_vec()
            .into_iter()
            .chain(SCORE_SUFFIX.iter().copied())
            .collect::<Vec<_>>();
        let value_key_bytes = k[score_byte_len..]
            .to_vec()
            .into_iter()
            .chain(VALUE_SUFFIX.iter().copied())
            .collect::<Vec<_>>();

        let ksvec = IVec::from(score_key_bytes);
        let kvvec = IVec::from(value_key_bytes);

        key_tree.remove(ksvec).map_err(sled_err_into)?;
        key_tree.remove(kvvec).map_err(sled_err_into)?;
        score_tree.remove(k).map_err(sled_err_into)?;
        removed += 1;
    }

    if key_tree.is_empty() {
        db.drop_tree(key_tree_bytes).map_err(sled_err_into)?;
    }
    if score_tree.is_empty() {
        db.drop_tree(score_tree_bytes).map_err(sled_err_into)?;
    }

    Ok(removed)
}

#[rustler::nif]
fn zitercollectionrembyrangebyscore<'a>(
    db_resouce: rustler::Term<'a>,
    collection: rustler::Binary,
    min_score: u64,
    max_score: Option<u64>,
    limit: usize,
) -> NifResult<u64> {
    let dbr: rustler::ResourceArc<DbResource> = db_resouce.decode()?;
    let db = &dbr.db;
    let score_tree_bytes = SCORE_PREFIX
        .to_vec()
        .into_iter()
        .chain(collection.as_slice().iter().copied())
        .collect::<Vec<_>>();
    let key_tree_bytes = KEY_PREFIX
        .to_vec()
        .into_iter()
        .chain(collection.as_slice().iter().copied())
        .collect::<Vec<_>>();

    let score_tree: sled::Tree = db.open_tree(IVec::from(score_tree_bytes.clone())).unwrap();
    let key_tree: sled::Tree = db.open_tree(IVec::from(key_tree_bytes.clone())).unwrap();

    let min_bytes = min_score.to_be_bytes().to_vec();
    let score_byte_len = min_bytes.clone().len();

    let iter = if let Some(o) = max_score {
        score_tree
            .range(IVec::from(min_bytes)..IVec::from(o.to_be_bytes().to_vec()))
            .keys()
    } else {
        score_tree.range(IVec::from(min_bytes)..).keys()
    };

    let mut iter = iter.filter_map(|l| l.ok()).take(limit);

    let mut removed: u64 = 0;
    while let Some(k) = iter.next() {
        let score_key_bytes = k[score_byte_len..]
            .to_vec()
            .into_iter()
            .chain(SCORE_SUFFIX.iter().copied())
            .collect::<Vec<_>>();
        let value_key_bytes = k[score_byte_len..]
            .to_vec()
            .into_iter()
            .chain(VALUE_SUFFIX.iter().copied())
            .collect::<Vec<_>>();

        let ksvec = IVec::from(score_key_bytes);
        let kvvec = IVec::from(value_key_bytes);

        key_tree.remove(ksvec).map_err(sled_err_into)?;
        key_tree.remove(kvvec).map_err(sled_err_into)?;
        score_tree.remove(k).map_err(sled_err_into)?;
        removed += 1;
    }

    if key_tree.is_empty() {
        db.drop_tree(key_tree_bytes).map_err(sled_err_into)?;
    }
    if score_tree.is_empty() {
        db.drop_tree(score_tree_bytes).map_err(sled_err_into)?;
    }

    Ok(removed)
}

#[rustler::nif]
fn zrangebyscore<'a>(
    env: rustler::Env<'a>,
    db_resouce: rustler::Term<'a>,
    collection: rustler::Binary,
    min_score: u64,
    max_score: Option<u64>,
    offset: usize,
    limit: usize,
) -> NifResult<Vec<rustler::Binary<'a>>> {
    let dbr: rustler::ResourceArc<DbResource> = db_resouce.decode()?;
    let db = &dbr.db;
    let score_tree_bytes = SCORE_PREFIX
        .to_vec()
        .into_iter()
        .chain(collection.as_slice().iter().copied())
        .collect::<Vec<_>>();

    let score_tree: sled::Tree = db.open_tree(IVec::from(score_tree_bytes)).unwrap();

    let min_bytes = min_score.to_be_bytes().to_vec();
    let score_byte_len = min_bytes.clone().len();
    let iter = if let Some(o) = max_score {
        score_tree
            .range(IVec::from(min_bytes)..IVec::from(o.to_be_bytes().to_vec()))
            .keys()
    } else {
        score_tree.range(IVec::from(min_bytes)..).keys()
    };

    Ok(iter
        .filter_map(|l| l.ok())
        .skip(offset)
        .take(limit)
        .map(|result| make_binary(env, &result[score_byte_len..]))
        .collect::<Vec<_>>())
}

#[rustler::nif]
fn zrangebyprefixscore<'a>(
    env: rustler::Env<'a>,
    db_resouce: rustler::Term<'a>,
    collection: rustler::Binary,
    prefix: rustler::Binary,
    min_score: u64,
    max_score: Option<u64>,
    offset: usize,
    limit: usize,
) -> NifResult<Vec<rustler::Binary<'a>>> {
    let dbr: rustler::ResourceArc<DbResource> = db_resouce.decode()?;
    let db = &dbr.db;
    let key_tree_bytes = KEY_PREFIX
        .to_vec()
        .into_iter()
        .chain(collection.as_slice().iter().copied())
        .collect::<Vec<_>>();

    let key_tree: sled::Tree = db.open_tree(IVec::from(key_tree_bytes.clone())).unwrap();

    let iter = key_tree.scan_prefix(prefix.as_slice());

    Ok(iter
        .filter_map(|l| l.ok())
        .filter(|l| l.0.last() == SCORE_SUFFIX.last())
        .filter(|result| {
            let score = make_u64(&result.1);
            if let Some(o) = max_score {
                min_score <= score && score < o
            } else {
                min_score <= score
            }
        })
        .skip(offset)
        .take(limit)
        .map(|result| make_binary(env, &result.0[..(result.0.len() - 1)]))
        .collect::<Vec<_>>())
}

#[rustler::nif]
fn zexists<'a>(
    db_resouce: rustler::Term<'a>,
    collection: rustler::Binary,
    min_score: u64,
    max_score: Option<u64>,
) -> NifResult<bool> {
    let dbr: rustler::ResourceArc<DbResource> = db_resouce.decode()?;
    let db = &dbr.db;
    let score_tree_bytes = SCORE_PREFIX
        .to_vec()
        .into_iter()
        .chain(collection.as_slice().iter().copied())
        .collect::<Vec<_>>();

    let score_tree: sled::Tree = db.open_tree(IVec::from(score_tree_bytes)).unwrap();

    let min_bytes = min_score.to_be_bytes().to_vec();
    let iter = if let Some(o) = max_score {
        score_tree
            .range(IVec::from(min_bytes)..IVec::from(o.to_be_bytes().to_vec()))
            .keys()
    } else {
        score_tree.range(IVec::from(min_bytes)..).keys()
    };
    Ok(!iter
        .filter_map(|l| l.ok())
        .take(1)
        .collect::<Vec<_>>()
        .is_empty())
}

#[rustler::nif]
fn zgetbykey<'a>(
    env: rustler::Env<'a>,
    db_resouce: rustler::Term<'a>,
    collection: rustler::Binary,
    key: rustler::Binary,
    min_score: u64,
) -> NifResult<Option<(Option<rustler::Binary<'a>>, Option<u64>)>> {
    let dbr: rustler::ResourceArc<DbResource> = db_resouce.decode()?;
    let db = &dbr.db;
    let key_tree_bytes = KEY_PREFIX
        .to_vec()
        .into_iter()
        .chain(collection.as_slice().iter().copied())
        .collect::<Vec<_>>();

    let key_tree: sled::Tree = db.open_tree(IVec::from(key_tree_bytes)).unwrap();

    let value_key_bytes = key
        .as_slice()
        .to_vec()
        .into_iter()
        .chain(VALUE_SUFFIX.iter().copied())
        .collect::<Vec<_>>();
    let score_key_bytes = key
        .as_slice()
        .to_vec()
        .into_iter()
        .chain(SCORE_SUFFIX.iter().copied())
        .collect::<Vec<_>>();

    let ksvec = IVec::from(score_key_bytes);
    let kvvec = IVec::from(value_key_bytes);
    let value = key_tree.get(kvvec.clone()).map_err(sled_err_into)?;
    let score = key_tree.get(ksvec.clone()).map_err(sled_err_into)?;
    let score_dec = if let Some(s) = score {
        Some(make_u64(&s))
    } else {
        None
    };
    let value_dec = if let Some(v) = value {
        Some(make_binary(env, &v))
    } else {
        None
    };
    if !value_dec.is_none() || !score_dec.is_none() {
        if score_dec.unwrap_or(u64::MAX) >= min_score {
            Ok(Some((value_dec, score_dec)))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

#[rustler::nif]
fn zrem<'a>(
    db_resouce: rustler::Term<'a>,
    collection: rustler::Binary,
    key: rustler::Binary,
) -> NifResult<rustler::Atom> {
    let dbr: rustler::ResourceArc<DbResource> = db_resouce.decode()?;
    let db = &dbr.db;
    let key_tree_bytes = KEY_PREFIX
        .to_vec()
        .into_iter()
        .chain(collection.as_slice().iter().copied())
        .collect::<Vec<_>>();
    let score_tree_bytes = SCORE_PREFIX
        .to_vec()
        .into_iter()
        .chain(collection.as_slice().iter().copied())
        .collect::<Vec<_>>();
    let key_tree: sled::Tree = db.open_tree(IVec::from(key_tree_bytes.clone())).unwrap();
    let score_tree: sled::Tree = db.open_tree(IVec::from(score_tree_bytes.clone())).unwrap();

    let value_key_bytes = key
        .as_slice()
        .to_vec()
        .into_iter()
        .chain(VALUE_SUFFIX.iter().copied())
        .collect::<Vec<_>>();
    let score_key_bytes = key
        .as_slice()
        .to_vec()
        .into_iter()
        .chain(SCORE_SUFFIX.iter().copied())
        .collect::<Vec<_>>();

    let ksvec = IVec::from(score_key_bytes);
    let kvvec = IVec::from(value_key_bytes);
    key_tree.remove(kvvec.clone()).map_err(sled_err_into)?;
    let score = key_tree.get(ksvec.clone()).map_err(sled_err_into)?;

    if let Some(s) = score {
        let score_bytes = s
            .to_vec()
            .into_iter()
            .chain(key.as_slice().iter().copied())
            .collect::<Vec<_>>();
        score_tree.remove(score_bytes).map_err(sled_err_into)?;
    }

    key_tree.remove(ksvec.clone()).map_err(sled_err_into)?;

    if key_tree.is_empty() {
        db.drop_tree(key_tree_bytes).map_err(sled_err_into)?;
    }
    if score_tree.is_empty() {
        db.drop_tree(score_tree_bytes).map_err(sled_err_into)?;
    }

    Ok(atoms::ok())
}

#[rustler::nif]
fn rpush<'a>(
    db_resouce: rustler::Term<'a>,
    collection: rustler::Binary,
    value: rustler::Binary,
) -> NifResult<rustler::Atom> {
    let dbr: rustler::ResourceArc<DbResource> = db_resouce.decode()?;
    let db = &dbr.db;
    let list_tree_bytes = LIST_PREFIX
        .to_vec()
        .into_iter()
        .chain(collection.as_slice().iter().copied())
        .collect::<Vec<_>>();

    let list_tree: sled::Tree = db.open_tree(IVec::from(list_tree_bytes)).unwrap();
    let right_side_id = db.generate_id().map_err(sled_err_into)?;

    let list_key = ((right_side_id as i128 - i64::MAX as i128) as i64)
        .to_be_bytes()
        .to_vec();

    list_tree
        .insert(list_key, value.as_slice())
        .map_err(sled_err_into)?;

    Ok(atoms::ok())
}

#[rustler::nif]
fn lpush<'a>(
    db_resouce: rustler::Term<'a>,
    collection: rustler::Binary,
    value: rustler::Binary,
) -> NifResult<rustler::Atom> {
    let dbr: rustler::ResourceArc<DbResource> = db_resouce.decode()?;
    let db = &dbr.db;
    let list_tree_bytes = LIST_PREFIX
        .to_vec()
        .into_iter()
        .chain(collection.as_slice().iter().copied())
        .collect::<Vec<_>>();

    let list_tree: sled::Tree = db.open_tree(IVec::from(list_tree_bytes)).unwrap();
    let right_side_id = db.generate_id().map_err(sled_err_into)?;
    let key = -1 * ((right_side_id as i128 - i64::MAX as i128) as i64);

    let list_key = key.to_be_bytes().to_vec();

    list_tree
        .insert(list_key, value.as_slice())
        .map_err(sled_err_into)?;

    Ok(atoms::ok())
}

#[rustler::nif]
fn lpop<'a>(
    env: rustler::Env<'a>,
    db_resouce: rustler::Term<'a>,
    collection: rustler::Binary,
) -> NifResult<Option<rustler::Binary<'a>>> {
    let dbr: rustler::ResourceArc<DbResource> = db_resouce.decode()?;
    let db = &dbr.db;
    let list_tree_bytes = LIST_PREFIX
        .to_vec()
        .into_iter()
        .chain(collection.as_slice().iter().copied())
        .collect::<Vec<_>>();

    let list_tree: sled::Tree = db.open_tree(IVec::from(list_tree_bytes.clone())).unwrap();

    let maybe_left = list_tree.pop_min().map_err(sled_err_into)?;
    if list_tree.is_empty() {
        db.drop_tree(list_tree_bytes).map_err(sled_err_into)?;
    }
    match maybe_left {
        Some(elem) => Ok(Some(make_binary(env, &elem.1))),
        _ => Ok(None),
    }
}

#[rustler::nif]
fn rpop<'a>(
    env: rustler::Env<'a>,
    db_resouce: rustler::Term<'a>,
    collection: rustler::Binary,
) -> NifResult<Option<rustler::Binary<'a>>> {
    let dbr: rustler::ResourceArc<DbResource> = db_resouce.decode()?;
    let db = &dbr.db;
    let list_tree_bytes = LIST_PREFIX
        .to_vec()
        .into_iter()
        .chain(collection.as_slice().iter().copied())
        .collect::<Vec<_>>();

    let list_tree: sled::Tree = db.open_tree(IVec::from(list_tree_bytes.clone())).unwrap();

    let maybe_left = list_tree.pop_max().map_err(sled_err_into)?;
    if list_tree.is_empty() {
        db.drop_tree(list_tree_bytes).map_err(sled_err_into)?;
    }
    match maybe_left {
        Some(elem) => Ok(Some(make_binary(env, &elem.1))),
        _ => Ok(None),
    }
}

fn make_binary<'a>(env: rustler::Env<'a>, bytes: &[u8]) -> rustler::Binary<'a> {
    let mut bin = match rustler::OwnedBinary::new(bytes.len()) {
        Some(bin) => bin,
        None => panic!("binary term allocation fail"),
    };
    bin.as_mut_slice()
        .write_all(&bytes[..])
        .expect("memory copy of string failed");

    bin.release(env)
}

fn make_u64<'a>(bytes: &[u8]) -> u64 {
    let b = bytes.try_into().expect("Invalid number of bytes");
    u64::from_be_bytes(b)
}

fn load(env: rustler::Env, _info: rustler::Term) -> bool {
    rustler::resource!(DbResource, env);
    true
}

rustler::init!(
    "Elixir.SortedSetKV",
    [
        open,
        clear,
        zgetbykey,
        zrangebyscore,
        zrangebyprefixscore,
        zadd,
        zrem,
        zscore,
        zscoreupdate,
        zrembyrangebyscore,
        zexists,
        lpush,
        rpush,
        rpop,
        lpop
    ],
    load = load
);
