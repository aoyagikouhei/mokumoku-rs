#!/bin/bash
redis-cli set response_1 "$(cat response/1.json)"
redis-cli set response_2 "$(cat response/2.json)"