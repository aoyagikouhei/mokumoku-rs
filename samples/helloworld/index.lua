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