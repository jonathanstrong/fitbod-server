var crypto = require('crypto');

var body = JSON.stringify({
    user_id: '3a2cbc79-00e5-4598-a5b2-74c5059724af',
    kind: 'ping',
});

var secret = "6KQ1CMZGFP84mJoip2crsGw5HpBhctnQ6Zkpj4/pVEqx/enTKvvwjpp57Nq7JS9gqjxyM1PtXcEHJxC0gag+dA==";

// decode base64-encoded secret to raw bytes
var key = Buffer.from(secret, 'base64');

// create a sha256 hmac with the secret
var hmac = crypto.createHmac('sha256', key);

// sign the require message with the hmac
// and finally base64 encode the result
var sig = hmac.update(body).digest('base64'); // -> Fn7nQsY3UqVKVr1kL7O+yP7J7WSM660oaNbSq42Vy7A=

var expected_sig = "Fn7nQsY3UqVKVr1kL7O+yP7J7WSM660oaNbSq42Vy7A=";

// console.log('body', body);
// console.log('sig = ', sig);
// console.log('expected =', expected_sig);

var assert = require('assert');
assert.equal(secret, "6KQ1CMZGFP84mJoip2crsGw5HpBhctnQ6Zkpj4/pVEqx/enTKvvwjpp57Nq7JS9gqjxyM1PtXcEHJxC0gag+dA==");
assert.equal(key.length, 64);
assert.equal(body,'{"user_id":"3a2cbc79-00e5-4598-a5b2-74c5059724af","kind":"ping"}');
assert.equal(sig, expected_sig);
