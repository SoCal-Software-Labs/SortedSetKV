# SortedSetKv

A scored sorted set KV. Inspired by Redis's sorted sets, but quite different. Written for Elixir backed by Rust's sled database.

This is the foundation of building a KV with a TTL, however I left it open to be used like redis `zadd` with an optional value field and score field. This means you can use it like a set, use it like a KV, use it like a scored set, use it like a scored KV, or use it as a KV TTL. It is very versitile. And very fast. Because everything is local you can get 1-4 times faster speeds than using Redis.

Also because you control the TLL, it won't auto evict things. This is very different from Redis which will start evicting keys regardless if their TTL has past when it runs out of memory.

SortedSetKV is stored on disk and can grow beyond your RAM limit.

## Disclaimer

This is alpha software and the API can change in the future.

## Basic Usage
```elixir
{:ok, db} = SortedSetKV.open("mypath")
# Add a key to a set, with a value and a score and if to only add if if the score is greater than
:ok = SortedSetKV.zadd(db, "mycollection", "hello", "world", 42, true)
:ok = SortedSetKV.zadd(db, "mycollection", "foo", "bar", 420, true)
:ok = SortedSetKV.zadd(db, "mycollection", "noscore", "", nil, true)
:ok = SortedSetKV.zadd(db, "mycollection", "novalue", nil, 100, true)
# Returns wether it exists and its score
{true, 42} = SortedSetKV.zscore(db, "mycollection", "hello")
{true, 420} = SortedSetKV.zscore(db, "mycollection", "foo")
{true, nil} = SortedSetKV.zscore(db, "mycollection", "noscore")
{true, 100} = SortedSetKV.zscore(db, "mycollection", "novalue")

# A key must have a score or a value to exist:
:ok = SortedSetKV.zadd(db, "mycollection", "noexists", nil, nil, true)
{false, nil} = SortedSetKV.zscore(db, "mycollection", "noexists")
```

## Retrieving Values
```elixir
# Get a key with a minimum score
{value, score} = SortedSetKV.zgetbykey(db, "mycollection", "hello", 0)
# A key with a score lower than the minscore will return nil
nil = SortedSetKV.zgetbykey(db, "mycollection", "foo", 500)
# see if any keys exist with the score
true = SortedSetKV.zexists(db, "mycollection", 0, 500)
```

## Conditional Add
```elixir
:ok = SortedSetKV.zadd(db, "mycollection", "hello", "world", 42, true)
{"world", 42} = SortedSetKV.zgetbykey(db, "mycollection", "hello", 0)
:ok = SortedSetKV.zadd(db, "mycollection", "hello", "value2", 0, true)
# only adds if the score is greather than
{"world", 42} = SortedSetKV.zgetbykey(db, "mycollection", "hello", 0)

# Setting the value to false overrides this
:ok = SortedSetKV.zadd(db, "mycollection", "hello", "value2", 0, false)
{"world", 0} = SortedSetKV.zgetbykey(db, "mycollection", "hello", 0)
```

## Iterating keys with scores
```
offset = 0
limit = 100
["hello"] = SortedSetKV.zrangebyscore(db, "mycollection", 0, 50, offset, limit)
# Filter by prefix and score
["foo"] = SortedSetKV.zrangebyprefixscore(db, "mycollection", "fo", 0, 500, offset, limit)
```


## Removing Values

```elixir
# Remove key
:ok = SortedSetKV.zrem(db, "mycollection", "hello")
# Remove all keys by score and returns how many it deleted
_ = SortedSetKV.zrembyrangebyscore(db, "mycollection", 0, 500, limit)
```


## Queue

```
:ok = SortedSetKV.rpush(db, "mylist", "value")
"value" = SortedSetKV.lpop(db, "mylist")
nil = SortedSetKV.lpop(db, "mylist")
```

## TTL

If you use millisecond timestamps as the score, it behaves like a TTL.

```elixir
{:ok, db} = SortedSetKV.open("mypath")
# Add a key to a set, with a value and a score
:ok = SortedSetKV.zadd(db, "mycollection", "hello", "world", :os.system_time(:millisecond) + 5000)
# Get key only if it is in TTL
SortedSetKV.zgetbykey(db, "mycollection", "foo", :os.system_time(:millisecond))

# Clean up exipired Keys
SortedSetKV.zrembyrangebyscore(db, "mycollection", 0, :os.system_time(:millisecond))
```

You can use a GenServer like this to customize your TTL cleanup. Because Elixir executes all Rust Nifs on one thread, you will not want to block for very long. It is wise to only delete a few keys at a time.

```elixir
defmodule TTLCleanup do
    use GenServer
    require Logger

    @review_time 5_000

    def start_link(conn) do
      GenServer.start_link(__MODULE__, conn, [])
    end

    def init(conn) do
      Process.send_after(self(), :review_storage, @review_time)
      {:ok, conn}
    end

    def handle_info(:review_storage, conn) do
      Logger.debug("TTL cleanup")

      :ok = scan(conn, "collection1")
      :ok = scan(conn, "collection2")
      :ok = scan(conn, "collection3")

      Process.send_after(self(), :review_storage, @review_time)

      {:noreply, conn}
    end

    def scan(conn, collection) do
      new_agg =
        SortedSetKV.zrembyrangebyscore(conn, collection, 0, :os.system_time(:millisecond), 100)

      case new_agg do
        0 ->
          :ok

        v when is_number(v) and v >= 99 ->
          scan(conn, collection)
      end
    end
end
```