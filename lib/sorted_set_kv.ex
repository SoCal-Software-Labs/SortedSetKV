defmodule SortedSetKV do
  use Rustler,
    otp_app: :sorted_set_kv,
    crate: :sortedsetkv

  def open(_path), do: :erlang.nif_error(:nif_not_loaded)

  def zadd(_db, _collection, _key, _value, _score, _add_if_gt),
    do: :erlang.nif_error(:nif_not_loaded)

  def zscoreupdate(_db, _collection, _key, _score, _min_score),
    do: :erlang.nif_error(:nif_not_loaded)

  def zrangebyscore(_db, _collection, _min_score, _max_score, _offset, _limit),
    do: :erlang.nif_error(:nif_not_loaded)

  def zrangebyprefixscore(_db, _collection, _prefix, _min_score, _max_score, _offset, _limit),
    do: :erlang.nif_error(:nif_not_loaded)

  def zscore(_db, _collection, _key), do: :erlang.nif_error(:nif_not_loaded)

  def zrembyrangebyscore(_db, _collection, _min_score, _max_score, _limit),
    do: :erlang.nif_error(:nif_not_loaded)

  def zgetbykey(_db, _collection, _key, _min_score), do: :erlang.nif_error(:nif_not_loaded)
  def zrem(_db, _collection, _key), do: :erlang.nif_error(:nif_not_loaded)
  def lpush(_db, _collection, _value), do: :erlang.nif_error(:nif_not_loaded)
  def rpush(_db, _collection, _value), do: :erlang.nif_error(:nif_not_loaded)
  def lpop(_db, _collection), do: :erlang.nif_error(:nif_not_loaded)
  def rpop(_db, _collection), do: :erlang.nif_error(:nif_not_loaded)
end
