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
- **스키마 추출**: 
  - 특정 이슈 범위의 DDL 스크립트 추출
  - Unix pipeline 지원으로 유연한 활용 가능

## 동작 원리

### Bytebase와의 통합
`shelltide`는 Bytebase API를 활용하여 데이터베이스 마이그레이션을 자동화합니다. Bytebase의 다음 구성 요소들과 상호작용합니다:

- **Project**: 개발 팀이나 애플리케이션 단위로 구성되는 논리적 그룹
- **Instance**: 실제 데이터베이스 서버 연결 정보
- **Database**: Instance 내의 개별 데이터베이스
- **Issue**: 데이터베이스 변경 요청 (DDL/DML 스크립트 포함)
- **Sheet**: Issue에서 실행할 실제 SQL 스크립트
- **Changelog**: 실행 완료된 변경사항의 기록
- **Revision**: 데이터베이스의 특정 시점 상태를 나타내는 스냅샷

### 마이그레이션 추적 시스템
`shelltide`는 Bytebase의 **Revision** 시스템을 활용하여 마이그레이션 상태를 추적합니다:

1. **Revision 저장**: 각 데이터베이스의 Revision에는 `project-name#issue-number` 형식으로 마지막 적용된 이슈 번호가 저장됩니다
2. **버전 비교**: 마이그레이션 시 소스 환경과 대상 환경의 Revision을 비교하여 적용할 이슈 범위를 결정합니다
3. **증분 적용**: 대상 환경의 현재 버전보다 높은 이슈 번호만 마이그레이션 대상에 포함됩니다
4. **상태 업데이트**: 마이그레이션 완료 후 새로운 Revision을 생성하여 적용된 최종 버전을 기록합니다

### 안전한 마이그레이션 프로세스
1. **환경 상태 확인**: 
   - 소스 환경에서 최신 완료된(DONE) 이슈 번호 확인
   - 대상 환경의 현재 Revision에서 마지막 적용된 버전 확인
2. **변경사항 발견**: 
   - 대상의 현재 버전과 목표 버전 사이의 모든 Changelog 검색
   - 대상 데이터베이스에 해당하는 변경사항만 필터링
3. **SQL 검증**: 
   - 각 Changelog의 SQL을 Bytebase SQL 검사기로 사전 검증
   - 구문 오류, 스키마 충돌 등을 미리 감지
4. **순차적 실행**: 
   - 시간순으로 정렬된 Changelog를 하나씩 적용
   - 각 변경사항마다 Sheet → Plan → Issue → Rollout 워크플로우 실행
5. **Revision 업데이트**: 
   - 성공적으로 완료된 경우 목표 버전으로 Revision 생성
   - 부분 실패의 경우 마지막으로 성공한 이슈까지의 버전으로 업데이트

### 버전 관리 특징
- **선언적 마이그레이션**: 현재 상태가 아닌 원하는 목표 버전을 지정
- **멱등성**: 동일한 버전에 대해 여러 번 실행해도 안전
- **높은 버전 처리**: 적용할 마이그레이션이 없어도 Revision을 목표 버전으로 업데이트

### 환경 별칭 시스템
- **로컬 구성**: `~/.shelltide/config.json`에 환경 별칭과 Bytebase 매핑 정보 저장
- **간소화된 명령어**: `prod-instance/admin-db` 대신 `prod/admin`와 같은 짧은 별칭 사용
- **다중 환경 지원**: 개발, 스테이징, 프로덕션 등 여러 환경을 하나의 CLI로 관리

## 필수 요구사항

- [Rust](https://www.rust-lang.org/tools/install) (최신 안정 버전)
- [Cargo](https://doc.rust-lang.org/cargo/) (Rust와 함께 제공)
- 실행 중인 [Bytebase](https://www.bytebase.com/) 인스턴스

## 설치

### Git 저장소에서 설치

```sh
# 최신 버전을 Git에서 직접 설치
cargo install --git <repository-url> --locked
```

### 소스에서 빌드 및 설치

```sh
# 저장소 복제
git clone <repository-url>
cd shelltide

# 로컬에서 빌드 및 설치
cargo install --path . --locked
```

이렇게 설치하면 `shelltide` 명령어가 시스템 PATH에 추가되어 어디서든 사용할 수 있습니다.

### 수동 빌드 (개발용)

```sh
# 개발 빌드
cargo build

# 릴리스 빌드
cargo build --release
```
실행 파일은 `target/debug/shelltide` 또는 `target/release/shelltide`에 생성됩니다.

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

모든 환경의 각 데이터베이스 스키마별로 마이그레이션 상태를 확인합니다. 기본 소스 환경(default.source_env)을 참조점으로 사용하여 상태를 표시합니다.

```sh
# 모든 환경과 스키마의 상태 확인 (기본 소스 환경 제외)
shelltide status

# 특정 환경만 필터링 (해당 환경의 모든 데이터베이스)
shelltide status staging

# 특정 환경의 특정 데이터베이스만 확인
shelltide status staging/bridge
```

**출력 예시:**

전체 상태 확인 시 (`shelltide status`):
```
SCHEMA                 ENVIRONMENT     LATEST CHANGELOG    
---------------------- --------------- --------------------
prod-instance/admin    prod            #240                
prod-instance/bridge   prod            #240                
stage-instance/admin   staging         NOT EXIST           
stage-instance/bridge  staging         #244                

Reference environment: dev (latest issue: #245)
```

특정 환경 필터링 시 (`shelltide status staging`):
```
SCHEMA                 ENVIRONMENT     LATEST CHANGELOG    
---------------------- --------------- --------------------
stage-instance/admin   staging         NOT EXIST           
stage-instance/bridge  staging         #244                

Reference environment: dev (latest issue: #245)
```

특정 데이터베이스 확인 시 (`shelltide status staging/bridge`):
```
SCHEMA                 ENVIRONMENT     LATEST CHANGELOG    
---------------------- --------------- --------------------
stage-instance/bridge  staging         #244                

Reference environment: dev (latest issue: #245)
```

**상태 표시:**
- `UP TO DATE`: 기준 환경과 같거나 더 최신 버전
- `#숫자`: 해당 이슈 번호까지 적용됨
- `NOT EXIST`: 해당 환경에 데이터베이스가 존재하지 않음
- `NO VERSION`: 데이터베이스는 존재하지만 버전 정보 없음

기준 환경(Reference environment)의 최신 이슈 번호가 하단에 표시됩니다.

### 5. 마이그레이션

기본 소스 환경(default.source_env)에서 대상 환경으로 마이그레이션을 적용합니다. 소스 데이터베이스 이름과 대상을 `<env-name>/<database>` 형식으로 지정하고, `--to`로 버전을 지정합니다.

```sh
# source 환경의 mydb 데이터베이스를 staging 환경의 mydb로 특정 버전까지 마이그레이션
shelltide migrate mydb staging/mydb --to 244

# 사용 가능한 최신 버전으로 마이그레이션
shelltide migrate mydb prod/mydb --to LATEST
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

### 7. 스키마 추출

변경사항을 DDL 스크립트로 추출할 수 있습니다. MIGRATE 타입의 changelog만 추출되며, SQL 안전성을 위해 세미콜론이 자동으로 추가됩니다.

```sh
# 전체 changelog 스크립트 추출
shelltide extract staging/bridge

# 특정 이슈 범위만 추출
shelltide extract staging/bridge --from 100 --to 105

# 특정 시작점부터 최신까지
shelltide extract staging/bridge --from 50
```

**출력 예시:**
```sql
-- Schema changes from issue #100 to #105
-- Generated by shelltide on 2025-09-08

-- Issue #101: Add user table
-- Executed: 2025-08-15T10:30:00Z
CREATE TABLE users (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    name VARCHAR(255) NOT NULL
);

-- Issue #102: Add email column
-- Executed: 2025-08-16T14:20:00Z
ALTER TABLE users ADD COLUMN email VARCHAR(255);

-- Issue #105: Create index
-- Executed: 2025-08-17T09:15:00Z
CREATE INDEX idx_users_email ON users(email);
```

## 개발

```sh
# 테스트는 단일 스레드로 실행 해야 합니다.
cargo test -- --test-threads=1
```
