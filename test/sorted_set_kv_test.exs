defmodule SortedSetKvTest do
  use ExUnit.Case

  test "greets the world" do
    offset = 0
    limit = 100

    {:ok, db} = SortedSetKV.open("testdb")
    :ok = SortedSetKV.clear(db)

    # Add a key to a set, with a value and a score.
    # The last parameter tells to only add if the old score is less than new score.
    :ok = SortedSetKV.zadd(db, "mycollection", "hello", "world", 42, true)
    :ok = SortedSetKV.zadd(db, "mycollection", "foo", "bar", 420, true)
    :ok = SortedSetKV.zadd(db, "mycollection", "noscore", "", nil, true)
    :ok = SortedSetKV.zadd(db, "mycollection", "novalue", nil, 100, true)
    # Returns whether it exists and its score
    assert {true, 42} == SortedSetKV.zscore(db, "mycollection", "hello")
    assert {true, 420} == SortedSetKV.zscore(db, "mycollection", "foo")
    assert {true, nil} == SortedSetKV.zscore(db, "mycollection", "noscore")
    assert {true, 100} == SortedSetKV.zscore(db, "mycollection", "novalue")

    # A key must have a score or a value to exist:
    :ok = SortedSetKV.zadd(db, "mycollection", "noexists", nil, nil, true)
    assert {false, nil} == SortedSetKV.zscore(db, "mycollection", "noexists")

    # Get a key with a minimum score
    assert {"world", 42} == SortedSetKV.zgetbykey(db, "mycollection", "hello", 0)
    # A key with a score lower than the minscore will return nil
    assert nil == SortedSetKV.zgetbykey(db, "mycollection", "foo", 500)
    # see if any keys exist with the score
    assert true == SortedSetKV.zexists(db, "mycollection", 0, 500)

    :ok = SortedSetKV.zadd(db, "mycollection", "hello", "world", 42, true)
    assert {"world", 42} == SortedSetKV.zgetbykey(db, "mycollection", "hello", 0)
    :ok = SortedSetKV.zadd(db, "mycollection", "hello", "value2", 0, true)
    # only adds if the score is greather than
    assert {"world", 42} == SortedSetKV.zgetbykey(db, "mycollection", "hello", 0)

    :ok = SortedSetKV.zscoreupdate(db, "mycollection", "hello", 0, true)
    assert {"world", 42} == SortedSetKV.zgetbykey(db, "mycollection", "hello", 0)

    # Setting the value to false overrides this
    :ok = SortedSetKV.zadd(db, "mycollection", "hello", "value2", 10, false)
    assert {"value2", 10} == SortedSetKV.zgetbykey(db, "mycollection", "hello", 0)

    :ok = SortedSetKV.zscoreupdate(db, "mycollection", "hello", 0, false)
    assert {"value2", 0} == SortedSetKV.zgetbykey(db, "mycollection", "hello", 0)

    assert ["hello"] == SortedSetKV.zrangebyscore(db, "mycollection", 0, 50, offset, limit)
    # Filter by prefix and score
    assert ["foo"] ==
             SortedSetKV.zrangebyprefixscore(db, "mycollection", "fo", 0, 500, offset, limit)

    assert [] == SortedSetKV.zrangebyprefixscore(db, "mycollection", "fo", 0, 50, offset, limit)

    assert ["foo"] ==
             SortedSetKV.zrangebyprefixscore(db, "mycollection", "fo", 420, 421, offset, limit)

    :ok = SortedSetKV.zrem(db, "mycollection", "hello")

    assert nil == SortedSetKV.zgetbykey(db, "mycollection", "hello", 0)
    assert [] == SortedSetKV.zrangebyscore(db, "mycollection", 0, 50, offset, limit)
    assert {false, nil} == SortedSetKV.zscore(db, "mycollection", "hello")

    _ = SortedSetKV.zrembyrangebyscore(db, "mycollection", 0, 500, limit)
    assert nil == SortedSetKV.zgetbykey(db, "mycollection", "foo", 0)

    assert [] ==
             SortedSetKV.zrangebyprefixscore(db, "mycollection", "fo", 420, 421, offset, limit)

    :ok = SortedSetKV.rpush(db, "mylist", "value")
    assert "value" == SortedSetKV.lpop(db, "mylist")
    assert nil == SortedSetKV.lpop(db, "mylist")
    :ok = SortedSetKV.rpush(db, "mylist", "1")
    :ok = SortedSetKV.rpush(db, "mylist", "2")
    :ok = SortedSetKV.lpush(db, "mylist", "0")
    assert "0" == SortedSetKV.lpop(db, "mylist")
    assert "2" == SortedSetKV.rpop(db, "mylist")
  end
end
