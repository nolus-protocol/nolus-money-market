#!/bin/sh

RELEASE_VERSION=dev cargo each run -x -t ci -- ../lint.sh --profile dev --not-as-workspace
