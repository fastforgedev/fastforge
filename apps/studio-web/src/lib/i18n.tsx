import * as React from 'react'

export type Locale = 'en' | 'zh-CN' | 'ja' | 'ko'

const STORAGE_KEY = 'fastforge-studio-locale'

/**
 * Translations keyed by the English source text (`en` IS the key). Each entry
 * carries only the languages that differ — missing ones fall back to English.
 * A key with no entry at all is itself the English text.
 */
type Entry = Partial<Record<Locale, string>>

const messages: { [key: string]: Entry | undefined } = {
  'Configure native packaging files for your application.': {
    'zh-CN': '配置应用的原生打包文件。',
  },
  'Local packaging files': { 'zh-CN': '本地打包文件' },
  'Open this project in local Studio to edit its packaging files.': {
    'zh-CN': '在本地 Studio 中打开项目，即可编辑打包文件。',
  },
  'Package format': { 'zh-CN': '打包格式' },
  'Saved directly to your project': { 'zh-CN': '直接保存到项目目录' },
  'Debian / Ubuntu': { 'zh-CN': 'Debian / Ubuntu' },
  'Package metadata and dependencies for .deb installers.': {
    'zh-CN': '配置 .deb 安装包的元数据和依赖。',
  },
  'Arch Linux': { 'zh-CN': 'Arch Linux' },
  'Package metadata and dependencies for Pacman packages.': {
    'zh-CN': '配置 Pacman 软件包的元数据和依赖。',
  },
  'Linux desktop': { 'zh-CN': 'Linux 桌面启动器' },
  'Launcher shared by Linux package formats, including DEB, RPM, Pacman and AppImage.':
    {
      'zh-CN':
        'DEB、RPM、Pacman 和 AppImage 等 Linux 打包格式共用的桌面启动器。',
    },
  'Package name': { 'zh-CN': '软件包名称' },
  Architecture: { 'zh-CN': '架构' },
  Maintainer: { 'zh-CN': '维护者' },
  Section: { 'zh-CN': '分区' },
  Priority: { 'zh-CN': '优先级' },
  Homepage: { 'zh-CN': '主页' },
  Dependencies: { 'zh-CN': '依赖' },
  'Separate dependencies with commas.': { 'zh-CN': '多个依赖以逗号分隔。' },
  'First line: summary. Remaining lines: full description.': {
    'zh-CN': '第一行填写摘要，其余行填写详细描述。',
  },
  'Package base': { 'zh-CN': '基础包名称' },
  Licenses: { 'zh-CN': '许可证' },
  'Optional dependencies': { 'zh-CN': '可选依赖' },
  Groups: { 'zh-CN': '分组' },
  'One value per line.': { 'zh-CN': '每行填写一个值。' },
  'Entry type': { 'zh-CN': '启动器类型' },
  'Display name': { 'zh-CN': '显示名称' },
  'Generic name': { 'zh-CN': '通用名称' },
  'Launch command': { 'zh-CN': '启动命令' },
  Icon: { 'zh-CN': '图标' },
  'Separate values with semicolons, ending with a semicolon.': {
    'zh-CN': '多个值以分号分隔，并以分号结尾。',
  },
  'Startup notification': { 'zh-CN': '启动通知' },
  'Run in terminal': { 'zh-CN': '在终端运行' },
  'true, false, or a template variable.': {
    'zh-CN': '填写 true、false 或模板变量。',
  },
  'New file': { 'zh-CN': '新文件' },
  'Unsaved changes': { 'zh-CN': '未保存的更改' },
  'On disk': { 'zh-CN': '已保存到磁盘' },
  'Discard changes and reload': { 'zh-CN': '放弃更改并重新加载' },
  Reload: { 'zh-CN': '重新加载' },
  'Loading packaging file…': { 'zh-CN': '正在加载打包文件…' },
  'File preview': { 'zh-CN': '文件预览' },
  'Template variables, comments and other fields are preserved. Variables resolve when packaging.':
    { 'zh-CN': '保留模板变量、注释和其他字段，变量将在打包时替换。' },
  'Changes apply to future packages.': { 'zh-CN': '更改将在下次打包时生效。' },
  Saved: { 'zh-CN': '已保存' },
  'Saving…': { 'zh-CN': '正在保存…' },
  'Save changes': { 'zh-CN': '保存更改' },
  'Create file': { 'zh-CN': '创建文件' },
  'The file changed on disk. Copy your changes from the preview, then reload before saving.':
    {
      'zh-CN': '文件已被外部修改。请先从预览中复制你的更改，再重新加载后保存。',
    },
  'Build, package, and publish your apps with Fastforge Studio.': {
    'zh-CN': '使用 Fastforge Studio 构建、打包和发布应用。',
    ja: 'Fastforge Studio でアプリのビルド、パッケージ化、公開を行いましょう。',
    ko: 'Fastforge Studio로 앱을 빌드, 패키징, 게시하세요.',
  },
  Language: {
    'zh-CN': '语言',
    ja: '言語',
    ko: '언어',
  },
  'Toggle theme': {
    'zh-CN': '切换主题',
    ja: 'テーマを切り替え',
    ko: '테마 전환',
  },
  Appearance: {
    'zh-CN': '外观',
    ja: '外観',
    ko: '모양',
  },
  Light: {
    'zh-CN': '浅色',
    ja: 'ライト',
    ko: '라이트',
  },
  Dark: {
    'zh-CN': '深色',
    ja: 'ダーク',
    ko: '다크',
  },
  Theme: {
    'zh-CN': '主题',
    ja: 'テーマ',
    ko: '테마',
  },
  Default: {
    'zh-CN': '默认',
    ja: 'デフォルト',
    ko: '기본',
  },
  Documentation: {
    'zh-CN': '文档',
    ja: 'ドキュメント',
    ko: '문서',
  },
  'Sign out': {
    'zh-CN': '退出登录',
    ja: 'サインアウト',
    ko: '로그아웃',
  },
  Projects: {
    'zh-CN': '项目',
    ja: 'プロジェクト',
    ko: '프로젝트',
  },
  'Search projects…': {
    'zh-CN': '搜索项目…',
    ja: 'プロジェクトを検索…',
    ko: '프로젝트 검색…',
  },
  'No matching project': {
    'zh-CN': '没有匹配的项目',
    ja: '一致するプロジェクトはありません',
    ko: '일치하는 프로젝트가 없습니다',
  },
  'All projects': {
    'zh-CN': '所有项目',
    ja: 'すべてのプロジェクト',
    ko: '모든 프로젝트',
  },
  'Project Settings': {
    'zh-CN': '项目设置',
    ja: 'プロジェクト設定',
    ko: '프로젝트 설정',
  },
  'This machine': {
    'zh-CN': '本机',
    ja: 'このマシン',
    ko: '이 머신',
  },
  'Cloud runners': {
    'zh-CN': '云端运行器',
    ja: 'クラウドランナー',
    ko: '클라우드 러너',
  },
  Settings: {
    'zh-CN': '设置',
    ja: '設定',
    ko: '설정',
  },
  'Leave settings': {
    'zh-CN': '离开设置',
    ja: '設定を離れる',
    ko: '설정 나가기',
  },
  'Back to projects': {
    'zh-CN': '返回项目列表',
    ja: 'プロジェクト一覧に戻る',
    ko: '프로젝트 목록으로 돌아가기',
  },
  'Back to {name}': {
    'zh-CN': '返回 {name}',
    ja: '{name} に戻る',
    ko: '{name}(으)로 돌아가기',
  },
  Overview: {
    'zh-CN': '概览',
    ja: '概要',
    ko: '개요',
  },
  Build: {
    'zh-CN': '构建',
    ja: 'ビルド',
    ko: '빌드',
  },
  Workflows: {
    'zh-CN': '工作流',
    ja: 'ワークフロー',
    ko: '워크플로',
  },
  Runs: {
    'zh-CN': '运行记录',
    ja: '実行履歴',
    ko: '실행 기록',
  },
  Artifacts: {
    'zh-CN': '产物',
    ja: '成果物',
    ko: '아티팩트',
  },
  Test: {
    'zh-CN': '测试',
    ja: 'テスト',
    ko: '테스트',
  },
  'Unit Tests': {
    'zh-CN': '单元测试',
    ja: '単体テスト',
    ko: '단위 테스트',
  },
  Distribute: {
    'zh-CN': '分发',
    ja: '配布',
    ko: '배포',
  },
  Stores: {
    'zh-CN': '应用商店',
    ja: 'ストア',
    ko: '앱 스토어',
  },
  Releases: {
    'zh-CN': '发布版本',
    ja: 'リリース',
    ko: '릴리스',
  },
  Project: {
    'zh-CN': '项目',
    ja: 'プロジェクト',
    ko: '프로젝트',
  },
  General: {
    'zh-CN': '常规',
    ja: '全般',
    ko: '일반',
  },
  'Build & Package': {
    'zh-CN': '构建与打包',
    ja: 'ビルドとパッケージ',
    ko: '빌드 및 패키지',
  },
  Environments: {
    'zh-CN': '环境',
    ja: '環境',
    ko: '환경',
  },
  Credentials: {
    'zh-CN': '凭据',
    ja: '認証情報',
    ko: '자격 증명',
  },
  Publishers: {
    'zh-CN': '发布渠道',
    ja: '配布チャネル',
    ko: '배포 채널',
  },
  Configuration: {
    'zh-CN': '配置',
    ja: '設定ファイル',
    ko: '구성',
  },
  Preferences: {
    'zh-CN': '偏好设置',
    ja: '環境設定',
    ko: '환경 설정',
  },
  Members: {
    'zh-CN': '成员',
    ja: 'メンバー',
    ko: '구성원',
  },
  'Access Tokens': {
    'zh-CN': '访问令牌',
    ja: 'アクセストークン',
    ko: '액세스 토큰',
  },
  Webhooks: {
    'zh-CN': 'Webhook',
    ja: 'Webhook',
    ko: 'Webhook',
  },
  Billing: {
    'zh-CN': '账单',
    ja: '請求',
    ko: '청구',
  },
  Workspace: {
    'zh-CN': '工作区',
    ja: 'ワークスペース',
    ko: '워크스페이스',
  },
  'On this machine': {
    'zh-CN': '在本机',
    ja: 'このマシン上',
    ko: '이 머신에서',
  },
  Hosted: {
    'zh-CN': '云端托管',
    ja: 'ホスト型',
    ko: '클라우드 호스팅',
  },
  'Workspace settings': {
    'zh-CN': '工作区设置',
    ja: 'ワークスペース設定',
    ko: '워크스페이스 설정',
  },
  'Add local project': {
    'zh-CN': '添加本地项目',
    ja: 'ローカルプロジェクトを追加',
    ko: '로컬 프로젝트 추가',
  },
  'New project': {
    'zh-CN': '新建项目',
    ja: '新規プロジェクト',
    ko: '새 프로젝트',
  },
  'No projects yet. Register one by running': {
    'zh-CN': '暂无项目。请在项目目录运行',
    ja: 'まだプロジェクトがありません。プロジェクトディレクトリで次を実行して登録してください',
    ko: '아직 프로젝트가 없습니다. 프로젝트 디렉터리에서 다음을 실행해 등록하세요',
  },
  'from a project directory.': {
    'zh-CN': '来注册项目。',
    ja: '（プロジェクトディレクトリから実行）',
    ko: '(프로젝트 디렉터리에서 실행)',
  },
  'Directory not found': {
    'zh-CN': '找不到目录',
    ja: 'ディレクトリが見つかりません',
    ko: '디렉터리를 찾을 수 없습니다',
  },
  'Can’t reach the Studio server': {
    'zh-CN': '无法连接到 Studio 服务器',
    ja: 'Studio サーバーに接続できません',
    ko: 'Studio 서버에 연결할 수 없습니다',
  },
  'Start it with': {
    'zh-CN': '请运行',
    ja: '次のコマンドで起動してください',
    ko: '다음으로 실행하세요',
  },
  'and reload.': {
    'zh-CN': '启动服务，然后重新加载页面。',
    ja: '実行後、ページを再読み込みしてください。',
    ko: '그리고 페이지를 다시 불러오세요.',
  },
  Connected: {
    'zh-CN': '已连接',
    ja: '接続済み',
    ko: '연결됨',
  },
  'Nothing connected yet.': {
    'zh-CN': '尚未连接任何渠道。',
    ja: 'まだ何も接続されていません。',
    ko: '아직 연결된 것이 없습니다.',
  },
  Available: {
    'zh-CN': '可用',
    ja: '利用可能',
    ko: '사용 가능',
  },
  Connect: {
    'zh-CN': '连接',
    ja: '接続',
    ko: '연결',
  },
  Ready: {
    'zh-CN': '就绪',
    ja: '準備完了',
    ko: '준비됨',
  },
  'Credentials missing': {
    'zh-CN': '缺少凭据',
    ja: '認証情報が不足しています',
    ko: '자격 증명 누락',
  },
  'Not configured': {
    'zh-CN': '未配置',
    ja: '未設定',
    ko: '구성되지 않음',
  },
  'set {names}': {
    'zh-CN': '设置 {names}',
    ja: '{names} を設定',
    ko: '{names} 설정',
  },
  unset: {
    'zh-CN': '未设置',
    ja: '未設定',
    ko: '설정되지 않음',
  },
  'Project not found': {
    'zh-CN': '找不到项目',
    ja: 'プロジェクトが見つかりません',
    ko: '프로젝트를 찾을 수 없습니다',
  },
  'What this project has shipped, and what is on the way.': {
    'zh-CN': '查看此项目已发布和即将发布的内容。',
    ja: 'このプロジェクトが公開したものと、今後公開予定のものを確認します。',
    ko: '이 프로젝트가 게시한 것과 곧 게시될 것을 확인합니다.',
  },
  'Run workflow': {
    'zh-CN': '运行工作流',
    ja: 'ワークフローを実行',
    ko: '워크플로 실행',
  },
  'Runs this week': {
    'zh-CN': '本周运行',
    ja: '今週の実行',
    ko: '이번 주 실행',
  },
  'Connected stores': {
    'zh-CN': '已连接商店',
    ja: '接続済みストア',
    ko: '연결된 스토어',
  },
  'Latest release': {
    'zh-CN': '最新发布',
    ja: '最新リリース',
    ko: '최신 릴리스',
  },
  Published: {
    'zh-CN': '已发布',
    ja: '公開済み',
    ko: '게시됨',
  },
  'In review': {
    'zh-CN': '审核中',
    ja: '審査中',
    ko: '검토 중',
  },
  'All releases': {
    'zh-CN': '所有发布版本',
    ja: 'すべてのリリース',
    ko: '모든 릴리스',
  },
  'Recent runs': {
    'zh-CN': '最近运行',
    ja: '最近の実行',
    ko: '최근 실행',
  },
  'No runs yet': {
    'zh-CN': '暂无运行记录',
    ja: 'まだ実行がありません',
    ko: '아직 실행 기록이 없습니다',
  },
  'Trigger a workflow to see it appear here.': {
    'zh-CN': '触发工作流后会在此处显示。',
    ja: 'ワークフローを実行すると、ここに表示されます。',
    ko: '워크플로를 실행하면 여기에 표시됩니다.',
  },
  'All runs': {
    'zh-CN': '所有运行记录',
    ja: 'すべての実行履歴',
    ko: '모든 실행 기록',
  },
  'Every build, package, publish and workflow execution, newest first.': {
    'zh-CN': '所有构建、打包、发布和工作流执行记录，按时间倒序排列。',
    ja: 'すべてのビルド、パッケージ、公開、ワークフロー実行を新しい順に表示します。',
    ko: '모든 빌드, 패키징, 게시, 워크플로 실행을 최신순으로 표시합니다.',
  },
  All: {
    'zh-CN': '全部',
    ja: 'すべて',
    ko: '모두',
  },
  Package: {
    'zh-CN': '打包',
    ja: 'パッケージ',
    ko: '패키지',
  },
  Publish: {
    'zh-CN': '发布',
    ja: '公開',
    ko: '게시',
  },
  Workflow: {
    'zh-CN': '工作流',
    ja: 'ワークフロー',
    ko: '워크플로',
  },
  Release: {
    'zh-CN': '发布',
    ja: 'リリース',
    ko: '릴리스',
  },
  Succeeded: {
    'zh-CN': '成功',
    ja: '成功',
    ko: '성공',
  },
  Failed: {
    'zh-CN': '失败',
    ja: '失敗',
    ko: '실패',
  },
  '12m ago': {
    'zh-CN': '12 分钟前',
    ja: '12分前',
    ko: '12분 전',
  },
  '1h ago': {
    'zh-CN': '1 小时前',
    ja: '1時間前',
    ko: '1시간 전',
  },
  '3h ago': {
    'zh-CN': '3 小时前',
    ja: '3時間前',
    ko: '3시간 전',
  },
  'Every package the packagers produced — APK, AAB, IPA, DMG, PKG, ZIP.': {
    'zh-CN': '打包器生成的所有产物，包括 APK、AAB、IPA、DMG、PKG 和 ZIP。',
    ja: 'パッケージャーが生成したすべての成果物（APK、AAB、IPA、DMG、PKG、ZIP）。',
    ko: '패키저가 생성한 모든 패키지 — APK, AAB, IPA, DMG, PKG, ZIP.',
  },
  'No artifacts yet': {
    'zh-CN': '暂无产物',
    ja: 'まだ成果物がありません',
    ko: '아직 아티팩트가 없습니다',
  },
  'Packages appear here once a run finishes. Each one can be analyzed and published from its detail page.': {
    'zh-CN': '运行完成后，软件包会显示在这里。可在详情页分析并发布每个软件包。',
    ja: '実行が完了すると、パッケージがここに表示されます。各パッケージは詳細ページから分析・公開できます。',
    ko: '실행이 완료되면 패키지가 여기에 표시됩니다. 각 패키지는 상세 페이지에서 분석하고 게시할 수 있습니다.',
  },
  'One row per version, showing where that version currently stands on each channel.': {
    'zh-CN': '每个版本一行，展示该版本在各渠道的当前状态。',
    ja: 'バージョンごとに 1 行表示し、各チャネルでの現在の状態を示します。',
    ko: '버전별로 한 행씩 표시되어 각 채널에서 해당 버전의 현재 상태를 보여줍니다.',
  },
  'No releases yet': {
    'zh-CN': '暂无发布版本',
    ja: 'まだリリースがありません',
    ko: '아직 릴리스가 없습니다',
  },
  'A release groups the artifacts of one version and tracks their status across every store and publisher.': {
    'zh-CN': '发布版本汇集同一版本的产物，并跟踪其在各应用商店和发布渠道的状态。',
    ja: 'リリースは 1 つのバージョンの成果物をまとめ、各ストア・配布チャネルでの状態を追跡します。',
    ko: '릴리스는 한 버전의 아티팩트를 모아 모든 스토어와 배포 채널에서 상태를 추적합니다.',
  },
  'The test suites defined for this project and their latest results.': {
    'zh-CN': '此项目定义的测试套件及其最新结果。',
    ja: 'このプロジェクトで定義されたテストスイートとその最新結果。',
    ko: '이 프로젝트에 정의된 테스트 스위트와 최신 결과.',
  },
  'No tests yet': {
    'zh-CN': '暂无测试',
    ja: 'まだテストがありません',
    ko: '아직 테스트가 없습니다',
  },
  'Test suites appear here once a run finishes. Each one can be analyzed from its detail page.': {
    'zh-CN': '运行完成后，测试套件会显示在这里，并可在详情页进行分析。',
    ja: '実行が完了するとテストスイートがここに表示され、詳細ページで分析できます。',
    ko: '실행이 완료되면 테스트 스위트가 여기에 표시되고 상세 페이지에서 분석할 수 있습니다.',
  },
  'New workflow': {
    'zh-CN': '新建工作流',
    ja: '新規ワークフロー',
    ko: '새 워크플로',
  },
  Run: {
    'zh-CN': '运行',
    ja: '実行',
    ko: '실행',
  },
  'Android package': {
    'zh-CN': 'Android 打包',
    ja: 'Android パッケージ',
    ko: 'Android 패키지',
  },
  'The YAML under .fastforge/workflows. These are definitions — each execution shows up under Runs.': {
    'zh-CN': '.fastforge/workflows 下的 YAML 定义；每次执行都会显示在运行记录中。',
    ja: '.fastforge/workflows の YAML 定義です。実行ごとに「実行履歴」に表示されます。',
    ko: '.fastforge/workflows 아래의 YAML 정의입니다. 각 실행은 실행 기록에 표시됩니다.',
  },
  'Storefronts that own the listing: versions, tracks, review state and catalog metadata.': {
    'zh-CN': '管理应用信息的商店，包括版本、轨道、审核状态和目录元数据。',
    ja: 'アプリ情報を管理するストア：バージョン、トラック、審査状態、カタログメタデータ。',
    ko: '앱 정보를 관리하는 스토어: 버전, 트랙, 검토 상태 및 카탈로그 메타데이터.',
  },
  'No apps registered under this store.': {
    'zh-CN': '此商店下没有已注册的应用。',
    ja: 'このストアに登録されたアプリはありません。',
    ko: '이 스토어에 등록된 앱이 없습니다.',
  },
  'Not pulled': {
    'zh-CN': '尚未拉取',
    ja: '未取得',
    ko: '가져오지 않음',
  },
  '{count} catalog files': {
    'zh-CN': '{count} 个目录文件',
    ja: 'カタログ {count} ファイル',
    ko: '카탈로그 파일 {count}개',
  },
  'General settings': {
    'zh-CN': '常规设置',
    ja: '一般設定',
    ko: '일반 설정',
  },
  'Project name, identifiers and the location Fastforge reads its config from.': {
    'zh-CN': '项目名称、标识符以及 Fastforge 读取配置的位置。',
    ja: 'プロジェクト名、識別子、Fastforge が設定を読み取る場所。',
    ko: '프로젝트 이름, 식별자, Fastforge가 구성을 읽는 위치.',
  },
  'Display name, application id and the path to this project on disk.': {
    'zh-CN': '显示名称、应用 ID 和项目在磁盘上的路径。',
    ja: '表示名、アプリケーション ID、ディスク上のプロジェクトへのパス。',
    ko: '표시 이름, 애플리케이션 ID, 디스크에서 이 프로젝트의 경로.',
  },
  'Which builder runs, and which package formats it produces.': {
    'zh-CN': '选择构建器及其生成的软件包格式。',
    ja: 'どのビルダーを実行し、どのパッケージ形式を生成するか。',
    ko: '어떤 빌더를 실행하고 어떤 패키지 형식을 생성하는지.',
  },
  'Builders and packagers': {
    'zh-CN': '构建器和打包器',
    ja: 'ビルダーとパッケージャー',
    ko: '빌더 및 패키저',
  },
  'Flavors, build variables and the values that differ per environment.': {
    'zh-CN': '不同环境的变体、构建变量和对应值。',
    ja: '環境ごとに異なるフレーバー、ビルド変数、その値。',
    ko: '환경마다 다른 플레이버, 빌드 변수 및 값.',
  },
  'No environments configured': {
    'zh-CN': '尚未配置环境',
    ja: '環境が設定されていません',
    ko: '구성된 환경이 없습니다',
  },
  'Signing keys and the API credentials stores and publishers authenticate with.': {
    'zh-CN': '签名密钥，以及应用商店和发布渠道用于身份验证的 API 凭据。',
    ja: '署名キー、そしてストアと配布チャネルが認証に使用する API 認証情報。',
    ko: '서명 키, 그리고 스토어와 배포 채널이 인증에 사용하는 API 자격 증명.',
  },
  'No credentials stored': {
    'zh-CN': '尚未保存凭据',
    ja: '認証情報が保存されていません',
    ko: '저장된 자격 증명이 없습니다',
  },
  'Plan, build minutes and invoices.': {
    'zh-CN': '套餐、构建时长和发票。',
    ja: 'プラン、ビルド時間、請求書。',
    ko: '요금제, 빌드 시간 및 청구서.',
  },
  'Current plan, usage against the included build minutes, and past invoices.': {
    'zh-CN': '当前套餐、已包含构建时长的使用情况及历史发票。',
    ja: '現在のプラン、含まれるビルド時間の使用量、過去の請求書。',
    ko: '현재 요금제, 포함된 빌드 시간 사용량 및 지난 청구서.',
  },
  'Appearance, language and notification defaults for this workspace.': {
    'zh-CN': '此工作区的外观、语言和默认通知设置。',
    ja: 'このワークスペースの外観、言語、通知のデフォルト設定。',
    ko: '이 워크스페이스의 외관, 언어 및 알림 기본값.',
  },
  'Theme, locale and how Studio notifies you when a run finishes.': {
    'zh-CN': '主题、区域设置，以及运行完成时 Studio 的通知方式。',
    ja: 'テーマ、地域設定、実行完了時に Studio が通知する方法。',
    ko: '테마, 로케일, 그리고 실행 완료 시 Studio가 알리는 방법.',
  },
  'Who has access to this workspace, and what they can do.': {
    'zh-CN': '管理工作区成员及其权限。',
    ja: 'ワークスペースへのアクセス権を持つメンバーと、その権限。',
    ko: '이 워크스페이스에 액세스할 수 있는 사람과 그들의 권한.',
  },
  'Invite your team': {
    'zh-CN': '邀请团队成员',
    ja: 'チームを招待する',
    ko: '팀 초대',
  },
  'Members are granted roles per workspace and inherit access to every project inside it.': {
    'zh-CN': '成员按工作区分配角色，并继承其中每个项目的访问权限。',
    ja: 'メンバーにはワークスペース単位でロールが付与され、その中のすべてのプロジェクトへのアクセスを継承します。',
    ko: '구성원은 워크스페이스별로 역할이 부여되며 그 안의 모든 프로젝트에 대한 액세스를 상속합니다.',
  },
  'Tokens the CLI and CI pipelines use to talk to this workspace.': {
    'zh-CN': 'CLI 和 CI 流水线用于访问此工作区的令牌。',
    ja: 'CLI と CI パイプラインがこのワークスペースと通信するためのトークン。',
    ko: 'CLI 및 CI 파이프라인이 이 워크스페이스와 통신하는 데 사용하는 토큰.',
  },
  'No tokens yet': {
    'zh-CN': '暂无令牌',
    ja: 'まだトークンがありません',
    ko: '아직 토큰이 없습니다',
  },
  'Create a token to let `fastforge` authenticate from GitHub Actions, GitLab CI or a local shell.': {
    'zh-CN': '创建令牌，让 `fastforge` 从 GitHub Actions、GitLab CI 或本地终端完成身份验证。',
    ja: '`fastforge` が GitHub Actions、GitLab CI、またはローカルシェルから認証できるようにトークンを作成します。',
    ko: '`fastforge`가 GitHub Actions, GitLab CI 또는 로컬 셸에서 인증할 수 있도록 토큰을 만듭니다.',
  },
  'Outbound notifications when runs and releases change state.': {
    'zh-CN': '运行和发布版本状态变化时发送通知。',
    ja: '実行やリリースの状態が変わったときに送信される通知。',
    ko: '실행 및 릴리스 상태가 변경될 때 전송되는 알림.',
  },
  'No webhooks configured': {
    'zh-CN': '尚未配置 Webhook',
    ja: 'Webhook が設定されていません',
    ko: '구성된 Webhook이 없습니다',
  },
  'Point run and release events at Slack, Lark or any HTTP endpoint.': {
    'zh-CN': '将运行和发布事件发送到 Slack、飞书或任意 HTTP 端点。',
    ja: '実行とリリースのイベントを Slack、Lark、または任意の HTTP エンドポイントに送信します。',
    ko: '실행 및 릴리스 이벤트를 Slack, Lark 또는 모든 HTTP 엔드포인트로 보냅니다.',
  },
  'The raw .fastforge/config.yaml. Every form on the other pages is a view onto this file.': {
    'zh-CN': '原始 .fastforge/config.yaml 文件；其他页面的表单都是此文件的视图。',
    ja: '生の .fastforge/config.yaml ファイル。他のページのフォームはすべてこのファイルのビューです。',
    ko: '원본 .fastforge/config.yaml 파일. 다른 페이지의 모든 폼은 이 파일의 뷰입니다.',
  },
  'Upload targets for finished artifacts. They have no listing and no review state, so they are configuration rather than something to check on.': {
    'zh-CN': '已完成产物的上传目标。它们没有应用信息或审核状态，因此属于配置项。',
    ja: '完成した成果物のアップロード先。アプリ情報や審査状態がないため、確認対象ではなく設定項目です。',
    ko: '완성된 아티팩트의 업로드 대상. 앱 정보나 검토 상태가 없으므로 확인 대상이 아닌 구성 항목입니다.',
  },
  'Run {id}': {
    'zh-CN': '运行 #{id}',
    ja: '実行 #{id}',
    ko: '실행 #{id}',
  },
  'Step timeline, logs and the artifacts this run produced.': {
    'zh-CN': '查看步骤时间线、日志和此次运行生成的产物。',
    ja: 'ステップのタイムライン、ログ、この実行が生成した成果物。',
    ko: '단계 타임라인, 로그, 이 실행이 생성한 아티팩트.',
  },
  'Run log': {
    'zh-CN': '运行日志',
    ja: '実行ログ',
    ko: '실행 로그',
  },
  'Streaming step output and per-step timings will render here.': {
    'zh-CN': '此处将显示实时步骤输出和各步骤耗时。',
    ja: 'ここにリアルタイムのステップ出力と、ステップごとの所要時間が表示されます。',
    ko: '여기에 실시간 단계 출력과 단계별 소요 시간이 표시됩니다.',
  },
  'Define staging, production and any custom flavors, along with the build arguments each one passes.': {
    'zh-CN': '定义预发布、生产及自定义变体，并设置各环境传递的构建参数。',
    ja: 'ステージング、本番、その他のカスタムフレーバーと、それぞれが渡すビルド引数を定義します。',
    ko: '스테이징, 프로덕션 및 사용자 지정 플레이버와 각각이 전달하는 빌드 인수를 정의합니다.',
  },
  'An editor for the file itself, so anything the forms do not cover stays reachable.': {
    'zh-CN': '直接编辑配置文件，确保表单未覆盖的选项仍可配置。',
    ja: 'ファイル自体を編集するエディタ。フォームでカバーされない項目も引き続き設定できます。',
    ko: '파일 자체를 편집하는 에디터. 폼에서 다루지 않는 항목도 계속 구성할 수 있습니다.',
  },
  'Keystores, App Store Connect keys and service accounts are referenced by name from the configuration.': {
    'zh-CN': '密钥库、App Store Connect 密钥和服务账号通过配置中的名称引用。',
    ja: 'キーストア、App Store Connect キー、サービスアカウントは設定から名前で参照されます。',
    ko: '키스토어, App Store Connect 키 및 서비스 계정은 구성에서 이름으로 참조됩니다.',
  },
  'Flutter, Gradle, Xcode or a custom builder, and the APK / AAB / IPA / DMG / PKG targets built from it.': {
    'zh-CN': '配置 Flutter、Gradle、Xcode 或自定义构建器，以及 APK、AAB、IPA、DMG、PKG 构建目标。',
    ja: 'Flutter、Gradle、Xcode、またはカスタムビルダー、そしてそこから生成される APK / AAB / IPA / DMG / PKG ターゲット。',
    ko: 'Flutter, Gradle, Xcode 또는 사용자 지정 빌더, 그리고 여기서 생성되는 APK / AAB / IPA / DMG / PKG 대상.',
  },
  'Credentials and target options for this publisher, plus what has been uploaded to it.': {
    'zh-CN': '此发布渠道的凭据、目标选项及上传记录。',
    ja: 'この配布チャネルの認証情報とターゲットオプション、そしてアップロード済みの内容。',
    ko: '이 배포 채널의 자격 증명과 대상 옵션, 그리고 업로드된 내용.',
  },
  'Publisher target': {
    'zh-CN': '发布目标',
    ja: '配布先',
    ko: '배포 대상',
  },
  'Target options, the credential it authenticates with, and its upload history.': {
    'zh-CN': '目标选项、身份验证凭据和上传历史。',
    ja: 'ターゲットオプション、認証に使用する認証情報、アップロード履歴。',
    ko: '대상 옵션, 인증에 사용하는 자격 증명, 업로드 기록.',
  },
  'Contents, signing and size breakdown, plus the publish targets it can go to.': {
    'zh-CN': '查看内容、签名、大小明细及可用发布目标。',
    ja: '内容、署名、サイズの内訳、そして配布可能な公開ターゲット。',
    ko: '콘텐츠, 서명 및 크기 분석, 그리고 배포할 수 있는 게시 대상.',
  },
  'Package analysis': {
    'zh-CN': '软件包分析',
    ja: 'パッケージ分析',
    ko: '패키지 분석',
  },
  'The output of `fastforge analyze` — manifest, permissions, signing and size — belongs on this page.': {
    'zh-CN': '此页面展示 `fastforge analyze` 的输出，包括清单、权限、签名和大小。',
    ja: 'このページには `fastforge analyze` の出力（マニフェスト、権限、署名、サイズ）が表示されます。',
    ko: '`fastforge analyze`의 출력(매니페스트, 권한, 서명, 크기)이 이 페이지에 표시됩니다.',
  },
  'Artifacts, changelog and per-channel rollout state for this version.': {
    'zh-CN': '此版本的产物、更新日志和各渠道发布状态。',
    ja: 'このバージョンの成果物、更新履歴、チャネルごとのロールアウト状態。',
    ko: '이 버전의 아티팩트, 변경 로그 및 채널별 롤아웃 상태.',
  },
  'Release detail': {
    'zh-CN': '发布详情',
    ja: 'リリース詳細',
    ko: '릴리스 상세',
  },
  'Channel-by-channel status, the artifacts attached to this version and its release notes.': {
    'zh-CN': '各渠道状态、此版本关联的产物及发布说明。',
    ja: 'チャネルごとの状態、このバージョンに紐づく成果物、リリースノート。',
    ko: '채널별 상태, 이 버전에 연결된 아티팩트 및 릴리스 노트.',
  },
  'Definition, dispatch inputs and run history for this workflow.': {
    'zh-CN': '此工作流的定义、触发输入和运行历史。',
    ja: 'このワークフローの定義、実行入力、実行履歴。',
    ko: '이 워크플로의 정의, 실행 입력 및 실행 기록.',
  },
  'Workflow definition': {
    'zh-CN': '工作流定义',
    ja: 'ワークフロー定義',
    ko: '워크플로 정의',
  },
  'The YAML source, its workflow_dispatch inputs and a manual trigger will live here.': {
    'zh-CN': '此处将展示 YAML 源码、workflow_dispatch 输入和手动触发入口。',
    ja: 'ここに YAML ソース、workflow_dispatch 入力、手動トリガーが表示されます。',
    ko: '여기에 YAML 소스, workflow_dispatch 입력 및 수동 트리거가 표시됩니다.',
  },
  'Pull catalog': {
    'zh-CN': '拉取目录',
    ja: 'カタログを取得',
    ko: '카탈로그 가져오기',
  },
  'Credentials incomplete': {
    'zh-CN': '凭据不完整',
    ja: '認証情報が不完全です',
    ko: '자격 증명 불완전',
  },
  'Syncing this store will fail until these resolve.': {
    'zh-CN': '补齐这些凭据前，商店同步将失败。',
    ja: 'これらの認証情報を解決するまで、このストアの同期は失敗します。',
    ko: '이 자격 증명을 해결하기 전까지 이 스토어 동기화는 실패합니다.',
  },
  'No apps under this store': {
    'zh-CN': '此商店下没有应用',
    ja: 'このストアにはアプリがありません',
    ko: '이 스토어 아래에 앱이 없습니다',
  },
  'Nothing pulled yet': {
    'zh-CN': '尚未拉取内容',
    ja: 'まだ取得されたものはありません',
    ko: '아직 가져온 것이 없습니다',
  },
  Locale: {
    'zh-CN': '区域',
    ja: '地域',
    ko: '로케일',
  },
  Track: {
    'zh-CN': '轨道',
    ja: 'トラック',
    ko: '트랙',
  },
  Version: {
    'zh-CN': '版本',
    ja: 'バージョン',
    ko: '버전',
  },
  'Promotional text': {
    'zh-CN': '推广文本',
    ja: 'プロモーション文',
    ko: '프로모션 텍스트',
  },
  "What's New": {
    'zh-CN': '更新内容',
    ja: '新機能',
    ko: '새로운 기능',
  },
  Description: {
    'zh-CN': '描述',
    ja: '説明',
    ko: '설명',
  },
  Tracks: {
    'zh-CN': '轨道',
    ja: 'トラック',
    ko: '트랙',
  },
  'No releases': {
    'zh-CN': '暂无发布版本',
    ja: 'リリースはありません',
    ko: '릴리스가 없습니다',
  },
  Information: {
    'zh-CN': '信息',
    ja: '情報',
    ko: '정보',
  },
  Categories: {
    'zh-CN': '分类',
    ja: 'カテゴリ',
    ko: '카테고리',
  },
  Keywords: {
    'zh-CN': '关键词',
    ja: 'キーワード',
    ko: '키워드',
  },
  Copyright: {
    'zh-CN': '版权',
    ja: '著作権',
    ko: '저작권',
  },
  Marketing: {
    'zh-CN': '营销页面',
    ja: 'マーケティング',
    ko: '마케팅',
  },
  Support: {
    'zh-CN': '支持页面',
    ja: 'サポート',
    ko: '지원',
  },
  'Privacy policy': {
    'zh-CN': '隐私政策',
    ja: 'プライバシーポリシー',
    ko: '개인정보 처리방침',
  },
  'Store not found': {
    'zh-CN': '找不到应用商店',
    ja: 'ストアが見つかりません',
    ko: '앱 스토어를 찾을 수 없습니다',
  },
  'Firebase App Distribution': {
    'zh-CN': 'Firebase 应用分发',
    ja: 'Firebase App Distribution',
    ko: 'Firebase App Distribution',
  },
  'GitHub Releases': {
    'zh-CN': 'GitHub 发布版本',
    ja: 'GitHub Releases',
    ko: 'GitHub Releases',
  },
  'S3-compatible storage': {
    'zh-CN': 'S3 兼容存储',
    ja: 'S3 互換ストレージ',
    ko: 'S3 호환 스토리지',
  },
  Custom: {
    'zh-CN': '自定义',
    ja: 'カスタム',
    ko: '사용자 지정',
  },
  'Internal and tester distribution for Android and iOS.': {
    'zh-CN': '面向 Android 和 iOS 的内部及测试人员分发。',
    ja: 'Android と iOS 向けの内部およびテスター配布。',
    ko: 'Android 및 iOS용 내부 및 테스터 배포.',
  },
  'Attach artifacts to a tagged GitHub release.': {
    'zh-CN': '将产物附加到带标签的 GitHub 发布版本。',
    ja: 'タグ付き GitHub リリースに成果物を添付します。',
    ko: '태그된 GitHub 릴리스에 아티팩트를 첨부합니다.',
  },
  'Any bucket that speaks the S3 API.': {
    'zh-CN': '任意兼容 S3 API 的存储桶。',
    ja: 'S3 API に対応した任意のバケット。',
    ko: 'S3 API를 지원하는 모든 버킷.',
  },
  'Hosted install pages for Android and iOS builds.': {
    'zh-CN': '为 Android 和 iOS 构建提供托管安装页面。',
    ja: 'Android と iOS ビルド向けのホスト型インストールページ。',
    ko: 'Android 및 iOS 빌드용 호스팅 설치 페이지.',
  },
  'Publish artifacts alongside a Vercel deployment.': {
    'zh-CN': '随 Vercel 部署一起发布产物。',
    ja: 'Vercel デプロイと一緒に成果物を公開します。',
    ko: 'Vercel 배포와 함께 아티팩트를 게시합니다.',
  },
  'Upload an IPA without managing the listing.': {
    'zh-CN': '无需管理应用信息即可上传 IPA。',
    ja: 'アプリ情報を管理せずに IPA をアップロードします。',
    ko: '앱 정보 관리 없이 IPA를 업로드합니다.',
  },
  'Upload to Huawei AppGallery.': {
    'zh-CN': '上传到华为 AppGallery。',
    ja: 'Huawei AppGallery にアップロードします。',
    ko: 'Huawei AppGallery에 업로드합니다.',
  },
  'Run your own command to move the artifact.': {
    'zh-CN': '运行自定义命令来传输产物。',
    ja: '独自のコマンドを実行して成果物を移動します。',
    ko: '자체 명령을 실행하여 아티팩트를 이동합니다.',
  },
}

type I18nContextValue = {
  locale: Locale
  setLocale: (locale: Locale) => void
  t: (key: string, values?: Record<string, string | number>) => string
}

const I18nContext = React.createContext<I18nContextValue | null>(null)

function initialLocale(): Locale {
  if (typeof window === 'undefined') return 'en'
  const stored = window.localStorage.getItem(STORAGE_KEY)
  if (stored === 'en' || stored === 'zh-CN' || stored === 'ja' || stored === 'ko')
    return stored
  const language = (window.navigator.language || 'en').toLowerCase()
  if (language.startsWith('zh')) return 'zh-CN'
  if (language.startsWith('ja')) return 'ja'
  if (language.startsWith('ko')) return 'ko'
  return 'en'
}

function interpolate(value: string, values?: Record<string, string | number>) {
  if (!values) return value
  return value.replace(/\{(\w+)\}/g, (_, name: string) =>
    String(values[name] ?? `{${name}}`),
  )
}

export function translate(
  locale: Locale,
  key: string,
  values?: Record<string, string | number>,
) {
  const entry = messages[key]
  return interpolate((entry && entry[locale]) ?? key, values)
}

export function I18nProvider({ children }: { children: React.ReactNode }) {
  const [locale, setLocaleState] = React.useState<Locale>(initialLocale)

  const setLocale = React.useCallback((nextLocale: Locale) => {
    window.localStorage.setItem(STORAGE_KEY, nextLocale)
    setLocaleState(nextLocale)
  }, [])

  const t = React.useCallback(
    (key: string, values?: Record<string, string | number>) =>
      translate(locale, key, values),
    [locale],
  )

  React.useEffect(() => {
    document.documentElement.lang = locale
    document.title = 'Fastforge Studio'
    const description = document.querySelector<HTMLMetaElement>(
      'meta[name="description"]',
    )
    if (description) {
      description.content = t(
        'Build, package, and publish your apps with Fastforge Studio.',
      )
    }
  }, [locale, t])

  const value = React.useMemo(
    () => ({ locale, setLocale, t }),
    [locale, setLocale, t],
  )

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>
}

export function useI18n() {
  const value = React.useContext(I18nContext)
  if (!value) throw new Error('useI18n must be used inside I18nProvider')
  return value
}
