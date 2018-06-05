# MokuMoku-rs
MokuMoku-rs is Mock Server with Redis and Lua. The web application made by Rust. The parsing request and making response made by Lua with Redis.

```
cargo run -- redis://localhost:6379/0 samples/helloworld/index.lua
```

```
curl http://localhost:7878/aaa

{
    "c": "hello"
}
```

```
curl http://localhost:7878/bbb

{
    "c": "world"
}
```

```lua:index.lua
local decoded = cjson.decode(ARGV[1])
local path = decoded['uri']['path']
local response = {content_type='application/json', status_code=200}
local key
if path == '/aaa' then
    key = 'response_1'
else
    key = 'response_2'
end
response['body'] = redis.call('GET', key)
return cjson.encode(response)
```

## Lua ARGV[1]
```json
{
    "uri": {
        "path": "/aaa",
        "query": ""
    },
    "body": "",
    "headers": {
        "Host": "localhost:7878"
    },
    "method": "get"
}
```