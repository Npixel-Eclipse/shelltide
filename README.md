# shelltide

`shelltide`는 Bytebase API를 활용하여 데이터베이스 스키마 마이그레이션을 간소화하는 명령줄 인터페이스(CLI) 도구입니다. Git과 유사한 워크플로우를 제공하여 여러 환경에 걸쳐 데이터베이스 변경사항을 관리하고 적용할 수 있습니다.

## 기능

- **인증**: Bytebase 인스턴스에 안전하게 로그인
- **환경 관리**: 다양한 데이터베이스 환경(예: `dev`, `staging`, `prod`)을 사용하기 쉬운 별칭으로 등록 및 관리
- **구성 관리**: 기본 소스 환경과 같은 전역 설정으로 명령어 단순화
- **상태 확인**: 모든 구성된 환경의 현재 마이그레이션 상태를 한눈에 확인
- **안전한 마이그레이션**:
  - 대상 버전을 지정하여 선언적으로 마이그레이션 적용
  - 마이그레이션 적용 전 자동 SQL 검사로 오류 방지

## 동작 원리

### 마이그레이션 추적
- `shelltide`는 Bytebase 리비전을 사용하여 마이그레이션 상태를 추적합니다
- 각 데이터베이스 리비전에는 마지막으로 적용된 이슈 번호가 저장됩니다
- 마이그레이션 시 소스와 대상 리비전을 비교하여 적용할 이슈를 결정합니다
- 더 새로운 이슈(높은 이슈 번호)만 마이그레이션 계획에 포함됩니다

### 안전한 마이그레이션 프로세스
1. **상태 확인**: 소스와 대상 환경 간 최신 리비전 비교
2. **이슈 발견**: 대상의 마지막 리비전 이후 생성된 모든 이슈 검색
3. **SQL 검증**: 대기 중인 모든 이슈에 대해 Bytebase SQL 검사 실행
4. **계획 생성**: 검증된 SQL 문으로 마이그레이션 계획 생성
5. **실행**: 모든 검증이 통과한 경우에만 변경사항 적용

### 환경 별칭
- 환경은 Bytebase 프로젝트와 인스턴스에 매핑되는 로컬 별칭으로 저장됩니다
- 전체 프로젝트 경로 대신 짧은 이름을 사용하여 명령어를 단순화합니다
- 구성은 `~/.shelltide/config.json`에 로컬로 저장됩니다

## 필수 요구사항

- [Rust](https://www.rust-lang.org/tools/install) (최신 안정 버전)
- [Cargo](https://doc.rust-lang.org/cargo/) (Rust와 함께 제공)
- 실행 중인 [Bytebase](https://www.bytebase.com/) 인스턴스

## 설치 및 빌드

1. 저장소 복제:
   ```sh
   git clone <repository-url>
   cd shelltide
   ```

2. 프로젝트 빌드:
   ```sh
   cargo build --release
   ```
   실행 파일은 `target/release/shelltide`에 생성됩니다. 쉬운 접근을 위해 시스템의 `PATH`에 있는 디렉토리로 이동할 수 있습니다.

## 사용법

### 1. 로그인

먼저 Bytebase 인스턴스에 로그인하여 자격 증명을 안전하게 저장합니다. 서비스 계정과 서비스 키를 사용하세요.

```sh
shelltide login \
  --url "https://bytebase.example.com" \
  --service-account "your-sa@service.bytebase.com" \
  --service-key "<service-key-json-or-key>"
```

### 2. 환경 구성

Bytebase 프로젝트를 명명된 환경으로 등록합니다.
- env-name: 원하는 환경 이름
- project: project 이름
- instance: instance 이름
```sh
# 개발 환경 추가
shelltide env add <env-name> <project> <instance>

# 스테이징 환경 추가
shelltide env add <env-name> <project> <instance>
```

언제든지 구성된 환경을 목록으로 확인할 수 있습니다:
```sh
shelltide env list
```

### 3. 기본 구성 설정

migration의 기준이 되는 *기본 소스 환경*을 설정합니다.

```sh
shelltide config set default.source_env <env-name>
```

### 4. 상태 확인

각 환경에서 마지막으로 적용된 이슈의 개요를 확인합니다.

```sh
shelltide status
```
**출력 예시:**
```
ENVIRONMENT     LATEST ISSUE         
--------------- --------------------
dev             #125                
staging         #123                
```

### 5. 마이그레이션

소스에서 대상으로 마이그레이션을 적용합니다. 환경과 데이터베이스를 `<env-name>/<database>`로 지정하고, `--to`로 버전을 지정합니다.

```sh
# dev/mydb에서 staging/mydb로 특정 버전까지 마이그레이션
shelltide migrate dev/mydb staging/mydb --to 244

# 사용 가능한 최신 버전으로 마이그레이션
shelltide migrate dev/mydb prod/mydb --to LATEST
```
명령어는 대기 중인 이슈에 대해 SQL을 검증하고, 오류가 없는 경우에만 진행합니다.

### 6. 셸 자동완성

셸에서 명령줄 자동완성을 활성화하려면 `completion` 명령어를 사용하세요.

예를 들어, Zsh에서 자동완성을 설정하려면 `.zshrc` 파일에 다음을 추가하세요:
```sh
eval "$(shelltide completion zsh)"
```

지원되는 셸: `bash`, `elvish`, `fish`, `powershell`, `zsh`.

PowerShell 예시 (Windows):

```powershell
# 현재 세션만
shelltide completion powershell | Out-String | Invoke-Expression

# 프로필에 영구 저장
Add-Content $PROFILE 'shelltide completion powershell | Out-String | Invoke-Expression'
```

## 개발

```sh
# 테스트는 단일 스레드로 실행 해야 합니다.
cargo test -- --test-threads=1
```
