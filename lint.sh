#!/bin/sh

profile='dev'
workspace='--workspace'

if [ "${#}" -eq 1 ]
then
  features="${1}"
else
  features_set=0
  profile_set=0
  workspace_set=0

  while [ "${#}" -ne 0 ]
  do
    case "${1}" in
    ('--features')
      if [ "${features_set}" -eq 1 ]
      then
        echo "Features already set to '${features}'!"
        exit 1
      fi
      features_set=1
      features="${2}"
      shift 2
      ;;
    ('--profile')
      if [ "${profile_set}" -eq 1 ]
      then
        echo "Profile already set to '${profile}'!"
        exit 1
      fi
      profile_set=1
      profile="${2}"
      shift 2
      ;;
    ('--not-as-workspace')
      if [ "${workspace_set}" -eq 1 ]
      then
        echo "Workspace mode already set!"
        exit 1
      fi
      workspace_set=1
      workspace=''
      shift 1
      ;;
    esac
  done

  if [ "${features_set}" -eq 0 ]
  then
    echo 'Features are not set!'
    exit 1
  fi

  if [ "${profile_set}" -eq 0 ]
  then
    echo 'Profile is not set!'
    exit 1
  fi
fi

# -D warnings is set so we allow 'deprecated' lints to tolerate them
# remove '-A deprecated' to find out if we use any

# TODO discuss options about `clippy::large_enum_variant`
# shellcheck disable=SC2086
cargo clippy ${workspace} --profile ${profile} --all-targets \
  --features "${features}" -- -D warnings -D future-incompatible \
  -D nonstandard-style -D rust-2018-compatibility -D rust-2018-idioms \
  -D rust-2021-compatibility -D unused -D clippy::all \
  -D clippy::unwrap_used -D clippy::unwrap_in_result \
  -A clippy::large_enum_variant
