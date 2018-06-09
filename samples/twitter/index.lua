local json = cjson.decode(ARGV[1])
local path = json['uri']['path']
local response = {content_type='application/json', status_code=200}
local key
-- curl "http://localhost:7878/1.1/statuses/user_timeline.json?screen_name=aoyagikouhei"
if path == '/1.1/statuses/user_timeline.json' and json['uri']['query']['screen_name'][0] == 'aoyagikouhei' then
    key = 'response_1'
else
    key = 'response_2'
end
response['body'] = redis.call('GET', key)
return cjson.encode(response)