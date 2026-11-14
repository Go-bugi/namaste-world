#!/bin/sh
# namaste-world 훅 스크립트. 훅이 받은 입력(stdin)을 앱의 로컬 서버로 보낸다.
# 사용법: namaste-hook.sh <agent> <Event> <엔드포인트 파일>
#
# 에이전트의 동작을 바꾸지 않도록 아무것도 출력하지 않고, 앱이 꺼져 있거나 응답하지 않아도 항상 0으로 끝난다.
# 엔드포인트 파일은 셸로 실행하지 않고 KEY=VALUE 줄만 읽는다.

agent=$1
event=$2
endpoint=$3

# 보내지 않고 끝낼 때도 입력은 끝까지 읽어 에이전트가 쓰기 실패를 겪지 않게 한다
drain() {
  cat >/dev/null 2>&1
  exit 0
}

case "$agent" in claude | codex) ;; *) drain ;; esac
case "$event" in '' | *[!A-Za-z]*) drain ;; esac
[ -r "$endpoint" ] || drain

port=
token=
while IFS='=' read -r key value || [ -n "$key" ]; do
  # Windows 쪽에서 쓴 파일이면 줄 끝에 CR이 붙어 있다
  value=$(printf '%s' "$value" | tr -d '\r')
  case "$key" in
    NAMASTE_PORT) port=$value ;;
    NAMASTE_TOKEN) token=$value ;;
  esac
done <"$endpoint"

case "$port" in '' | *[!0-9]*) drain ;; esac
case "$token" in '' | *[!0-9a-f]*) drain ;; esac

url="http://127.0.0.1:$port/hook/$agent/$event"

# WSL2 기본 네트워크(NAT)에서는 배포판의 127.0.0.1이 Windows에 닿지 않는다.
# Windows 쪽 curl.exe를 interop로 부르면 Windows의 127.0.0.1로 간다. 없으면 배포판의 curl을 쓴다(mirrored 모드).
# curl.exe를 미리 실행해 보지 않는다. interop가 느리면 그것만으로 시간 상한을 넘기기 때문이다.
curl_bin=
if [ -n "${WSL_DISTRO_NAME:-}" ] || grep -qi microsoft /proc/sys/kernel/osrelease 2>/dev/null; then
  curl_bin=$(command -v curl.exe 2>/dev/null)
  if [ -z "$curl_bin" ] && [ -x /mnt/c/Windows/System32/curl.exe ]; then
    curl_bin=/mnt/c/Windows/System32/curl.exe
  fi
fi
if [ -z "$curl_bin" ]; then
  command -v curl >/dev/null 2>&1 || drain
  curl_bin=curl
fi

# interop가 멈춰도 2초 안에 끝나도록 coreutils timeout이 있으면 한 번 더 감싼다
limit=
if command -v timeout >/dev/null 2>&1; then
  limit="timeout 1.8"
fi

# 소수점 시간 값을 쉼표 로캘에서 잘못 읽지 않게 LC_NUMERIC=C로 둔다
LC_ALL= LC_NUMERIC=C $limit "$curl_bin" -s -o /dev/null \
  --connect-timeout 0.5 --max-time 1.3 \
  --noproxy 127.0.0.1 \
  -H "Content-Type: application/json" \
  -H "X-Namaste-Token: $token" \
  --data-binary @- \
  "$url" >/dev/null 2>&1

exit 0
