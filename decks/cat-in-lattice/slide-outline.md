# Cat-In-Lattice — 랄프톤 서울 중간발표

## Slide 1: 표지
- **프로젝트명:** Cat-In-Lattice
- **팀:** NomaDamas
- **GitHub:** github.com/NomaDamas/Cat-In-Lattice
- **한 줄 소개:** 에이전트가 일하는 동안, Ghostty 하단에서 살아 움직이며 상태·알림·유희를 제공하는 companion pane

## Slide 2: 문제 정의
- **시대 변화:** IDE가 아닌 터미널로, AI agent를 통해 코딩하는 시대
- **현실:** 화면을 쪼개 8개 세션을 동시에 orchestration — 하지만 대부분 잉여 공간
- **핵심 고통:**
  - 어디에 인간의 판단과 작업이 필요한지 알아차리기 어려움
  - 에이전트 작업 중 대기 시간이 생기지만, 그 시간을 활용할 방법이 없음
  - 터미널은 텍스트만 흘러가는 삭막한 공간 — 인간적 요소 부재
- **빈도:** 하루 수십 번, 매 에이전트 세션마다 반복

## Slide 3: 솔루션 — Cat-In-Lattice
- **한 문장:** Ghostty split pane에서 동작하는 픽셀아트 고양이 companion — 에이전트 상태 알림 + 타마고치 + 미니게임
- **핵심 기능:**
  - 🐱 픽셀아트 고양이: 5가지 감정, idle 애니메이션, 다마고치형 영속성
  - 📢 배너 시스템: Slack 공지, 명언, 에이전트 완료/에러 알림 (파일 워처)
  - 🎮 미니게임 4종: Pacman, Snake, Tetris, Breakout — 대기 시간 킬링타임
  - 📐 적응형 레이아웃: 기본 2패널(고양이|배너), 게임 시 3패널(고양이|게임|배너)
- **데모 흐름:** `cat-in-lattice` 실행 → 고양이 idle → 이벤트 팝업 → 쓰다듬기 → 게임 진입 → 에이전트 완료 알림

## Slide 4: Ralph 셋업 — 명세 주도 AI 개발
- **Ouroboros 인터뷰 → Seed 명세:**
  - Socratic interview 16라운드로 요구사항 결정화
  - 22개 acceptance criteria가 담긴 seed.yaml 생성
  - 모호도 점수 0.12 — 거의 모든 결정이 인터뷰에서 확정됨
- **Ralph 루프 9 iteration으로 완성:**
  - Iteration 1: 스캐폴딩 (3개 에이전트 병렬 → cat/banner/games 모듈 동시 생성)
  - Iteration 4: tokio → sync 아키텍처 전환 (기술 난점 자율 해결)
  - Iteration 8: 애니메이션 오버라이드 버그 발견 및 수정
  - 매 iteration마다 자동 QA → git commit+push
- **기술적 난점 해소:**
  - SuperLightTUI가 Rust 라이브러리임을 파악 → 자동으로 Rust 프로젝트로 전환
  - stdin 충돌 문제 → tokio 제거, std::thread 기반 sync 아키텍처로 리팩토링
  - 터미널 크기 엣지케이스 → 모든 게임에 최소 크기 가드 추가
- **도구:** Claude Opus 4.6 (1M context) + Ouroboros Ralph 루프

## Slide 5: 현재 진행 상황
- **완성도:**
  - ✅ 21 Rust 소스 파일, 4,677 LOC
  - ✅ 68 테스트 통과, 0 경고, 0 clippy 린트
  - ✅ 3.8MB 릴리즈 바이너리 (LTO + strip)
  - ✅ GitHub Actions CI (ubuntu + macos)
  - ✅ Homebrew formula 준비
  - ✅ README, LICENSE (MIT)
- **다음 단계:**
  - 실제 Ghostty split pane 통합 테스트
  - Homebrew tap 배포
  - 추가 고양이 스킨 및 커스텀 캐릭터 지원
