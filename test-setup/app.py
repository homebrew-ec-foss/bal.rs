import time
import sys

import redis
from flask import Flask

app = Flask(__name__)
db = redis.Redis(host="redis", port=6379)


def get_hit_count():
    retries = 5
    while True:
        try:
            return
        except redis.exceptions.ConnectionError as exc:
            if retries == 0:
                raise exc
            retries -= 1
            time.sleep(0.5)


def set_key(key, value):
    retries = 5
    while True:
        try:
            return db.set(key, value)
        except redis.exceptions.ConnectionError as exc:
            if retries == 0:
                raise exc
            retries -= 1
            time.sleep(0.5)


def get_key(key):
    retries = 5
    while True:
        try:
            return db.get(key)
        except redis.exceptions.ConnectionError as exc:
            if retries == 0:
                raise exc
            retries -= 1
            time.sleep(0.5)


@app.route("/set/<key>/<value>")
def set_key_handler(key, value):
    if set_key(key, value):
        return "Set key '{}' to value '{}'".format(key, value)


@app.route("/get/<key>")
def get_key_handler(key):
    try:
        value = get_key(key).decode("utf-8")
    except:
        return "Key not present"
    return "Retrieved key '{}' which has a value of '{}'".format(key, value)


@app.route("/")
def home():
    msg = r"Go to /set/[key]/[value] to store a [key] with [value], go to /get/[key] to retrieve the value associated with [key]"
    return msg


if __name__ == "__main__":
    port = int(sys.argv[1])
    app.run(debug=True, host="0.0.0.0", port=port)
