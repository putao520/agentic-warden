# iOS 应用开发规范 - CODING-STANDARDS-IOS

**版本**: 2.0.0
**适用范围**: iOS/iPadOS/watchOS/tvOS 原生应用开发
**技术栈**: Swift、SwiftUI、UIKit、Combine、Core Data、URLSession
**最后更新**: 2025-12-25

---

## 🚨 核心铁律（继承自 common.md）

> **必须遵循 common.md 的四大核心铁律**

```
铁律1: SPEC 是唯一真源（SSOT）
       - UI/UX 实现必须符合 SPEC 定义
       - 数据模型、API 接口以 SPEC 为准

铁律2: 智能复用与销毁重建
       - 现有组件完全匹配 → 直接复用
       - 部分匹配 → 删除重建，不做渐进式修改

铁律3: 禁止渐进式开发
       - 禁止在旧 ViewController 上添加新功能
       - 禁止保留废弃的 Storyboard 和 XIB

铁律4: Context7 调研先行
       - 使用 Apple 官方框架和成熟第三方库
       - 禁止自己实现网络、图片缓存等基础设施
```

---

## 🏗️ 项目架构

### Clean Architecture + MVVM-C
```
MyApp/
├── Application/             # App 生命周期
│   ├── AppDelegate.swift
│   ├── SceneDelegate.swift
│   └── AppCoordinator.swift
├── Domain/                  # 领域层（纯 Swift）
│   ├── Entities/            # 领域实体
│   ├── UseCases/            # 用例接口
│   └── Repositories/        # Repository 协议
├── Data/                    # 数据层
│   ├── Network/             # 网络层
│   │   ├── API/             # API 定义
│   │   ├── DTOs/            # 数据传输对象
│   │   └── NetworkService.swift
│   ├── Persistence/         # 持久化
│   │   ├── CoreData/        # Core Data 模型
│   │   └── UserDefaults/    # UserDefaults 封装
│   └── Repositories/        # Repository 实现
├── Presentation/            # 表现层
│   ├── Scenes/              # 页面模块
│   │   ├── Home/
│   │   ├── Profile/
│   │   └── Settings/
│   ├── Common/              # 共享 UI
│   │   ├── Views/
│   │   ├── ViewModifiers/
│   │   └── Components/
│   └── Coordinators/        # 导航协调器
├── Core/                    # 核心工具
│   ├── Extensions/
│   ├── Utilities/
│   └── Constants/
└── Resources/               # 资源文件
    ├── Assets.xcassets
    ├── Localizable.strings
    └── Info.plist
```

### 模块化架构（SPM）
```swift
// ✅ Package.swift 多模块配置
let package = Package(
    name: "MyApp",
    platforms: [.iOS(.v16)],
    products: [
        .library(name: "Core", targets: ["Core"]),
        .library(name: "Domain", targets: ["Domain"]),
        .library(name: "Data", targets: ["Data"]),
        .library(name: "UI", targets: ["UI"]),
    ],
    targets: [
        .target(name: "Core", dependencies: []),
        .target(name: "Domain", dependencies: ["Core"]),
        .target(name: "Data", dependencies: ["Domain", "Core"]),
        .target(name: "UI", dependencies: ["Domain", "Core"]),
    ]
)
```

---

## 📜 Swift 编码规范

### 协议导向编程
```swift
// ✅ 使用协议定义抽象
protocol UserRepository {
    func getUser(id: String) async throws -> User
    func saveUser(_ user: User) async throws
    func observeUser(id: String) -> AsyncStream<User>
}

// ✅ 协议扩展提供默认实现
extension UserRepository {
    func getUserOrNil(id: String) async -> User? {
        try? await getUser(id: id)
    }
}

// ✅ 协议组合
typealias DataRepository = UserRepository & PostRepository & CommentRepository

// ✅ 使用 some 关键字隐藏具体类型
func makeUserRepository() -> some UserRepository {
    UserRepositoryImpl(networkService: NetworkService.shared)
}
```

### 现代并发 (Swift Concurrency)
```swift
// ✅ Actor 保护共享状态
actor UserCache {
    private var cache: [String: User] = [:]

    func get(_ id: String) -> User? {
        cache[id]
    }

    func set(_ user: User) {
        cache[user.id] = user
    }

    func clear() {
        cache.removeAll()
    }
}

// ✅ TaskGroup 并发执行
func fetchAllData() async throws -> CombinedData {
    async let users = userRepository.getUsers()
    async let posts = postRepository.getPosts()
    async let comments = commentRepository.getComments()

    return try await CombinedData(
        users: users,
        posts: posts,
        comments: comments
    )
}

// ✅ AsyncStream 异步序列
func observeLocationUpdates() -> AsyncStream<CLLocation> {
    AsyncStream { continuation in
        let manager = CLLocationManager()
        let delegate = LocationDelegate { location in
            continuation.yield(location)
        }
        manager.delegate = delegate

        continuation.onTermination = { _ in
            manager.stopUpdatingLocation()
        }

        manager.startUpdatingLocation()
    }
}

// ✅ 结构化并发与取消
func downloadImages(urls: [URL]) async throws -> [UIImage] {
    try await withThrowingTaskGroup(of: (Int, UIImage).self) { group in
        for (index, url) in urls.enumerated() {
            group.addTask {
                let (data, _) = try await URLSession.shared.data(from: url)
                guard let image = UIImage(data: data) else {
                    throw ImageError.invalidData
                }
                return (index, image)
            }
        }

        var images = [Int: UIImage]()
        for try await (index, image) in group {
            images[index] = image
        }
        return urls.indices.compactMap { images[$0] }
    }
}
```

### 值类型优先
```swift
// ✅ 使用 struct 而非 class
struct User: Identifiable, Codable, Hashable {
    let id: String
    var name: String
    var email: String
    var avatarURL: URL?
    var createdAt: Date

    // 使用 CodingKeys 自定义映射
    enum CodingKeys: String, CodingKey {
        case id
        case name
        case email
        case avatarURL = "avatar_url"
        case createdAt = "created_at"
    }
}

// ✅ Copy-on-Write 语义
struct LargeData {
    private var storage: Storage

    private class Storage {
        var data: [Int]
        init(_ data: [Int]) { self.data = data }
    }

    init(_ data: [Int]) {
        storage = Storage(data)
    }

    var data: [Int] {
        get { storage.data }
        set {
            if !isKnownUniquelyReferenced(&storage) {
                storage = Storage(newValue)
            } else {
                storage.data = newValue
            }
        }
    }
}
```

---

## 🎨 SwiftUI

### 架构模式
```swift
// ✅ 状态管理
@MainActor
final class UserViewModel: ObservableObject {
    @Published private(set) var state: ViewState<User> = .idle

    private let getUserUseCase: GetUserUseCase

    init(getUserUseCase: GetUserUseCase) {
        self.getUserUseCase = getUserUseCase
    }

    func loadUser(id: String) {
        state = .loading

        Task {
            do {
                let user = try await getUserUseCase.execute(id: id)
                state = .loaded(user)
            } catch {
                state = .error(error)
            }
        }
    }
}

enum ViewState<T> {
    case idle
    case loading
    case loaded(T)
    case error(Error)
}

// ✅ View 组织
struct UserScreen: View {
    @StateObject private var viewModel: UserViewModel

    init(getUserUseCase: GetUserUseCase) {
        _viewModel = StateObject(wrappedValue: UserViewModel(getUserUseCase: getUserUseCase))
    }

    var body: some View {
        UserContentView(
            state: viewModel.state,
            onRetry: { viewModel.loadUser(id: "1") }
        )
        .task {
            viewModel.loadUser(id: "1")
        }
    }
}

// ✅ 无状态 View（可测试）
struct UserContentView: View {
    let state: ViewState<User>
    let onRetry: () -> Void

    var body: some View {
        switch state {
        case .idle:
            EmptyView()
        case .loading:
            ProgressView()
        case .loaded(let user):
            UserDetailView(user: user)
        case .error(let error):
            ErrorView(error: error, onRetry: onRetry)
        }
    }
}
```

### 性能优化
```swift
// ✅ 使用 @ViewBuilder 优化条件渲染
struct ContentView: View {
    let isLoggedIn: Bool

    var body: some View {
        content
    }

    @ViewBuilder
    private var content: some View {
        if isLoggedIn {
            MainTabView()
        } else {
            LoginView()
        }
    }
}

// ✅ 使用 EquatableView 优化重绘
struct ExpensiveView: View, Equatable {
    let data: ExpensiveData

    var body: some View {
        // 复杂渲染
    }

    static func == (lhs: ExpensiveView, rhs: ExpensiveView) -> Bool {
        lhs.data.id == rhs.data.id
    }
}

// ✅ 使用 LazyVStack/LazyHStack
struct ItemListView: View {
    let items: [Item]

    var body: some View {
        ScrollView {
            LazyVStack(spacing: 16) {
                ForEach(items) { item in
                    ItemRow(item: item)
                        .id(item.id)  // 优化 diff
                }
            }
        }
    }
}

// ✅ 自定义 PreferenceKey
struct SizePreferenceKey: PreferenceKey {
    static var defaultValue: CGSize = .zero
    static func reduce(value: inout CGSize, nextValue: () -> CGSize) {
        value = nextValue()
    }
}

extension View {
    func readSize(onChange: @escaping (CGSize) -> Void) -> some View {
        background(
            GeometryReader { geometry in
                Color.clear
                    .preference(key: SizePreferenceKey.self, value: geometry.size)
            }
        )
        .onPreferenceChange(SizePreferenceKey.self, perform: onChange)
    }
}
```

### 自定义组件
```swift
// ✅ 自定义 ViewModifier
struct CardStyle: ViewModifier {
    func body(content: Content) -> some View {
        content
            .padding()
            .background(Color(.systemBackground))
            .cornerRadius(12)
            .shadow(color: .black.opacity(0.1), radius: 8, x: 0, y: 4)
    }
}

extension View {
    func cardStyle() -> some View {
        modifier(CardStyle())
    }
}

// ✅ 自定义 ButtonStyle
struct PrimaryButtonStyle: ButtonStyle {
    @Environment(\.isEnabled) private var isEnabled

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.headline)
            .foregroundColor(.white)
            .frame(maxWidth: .infinity)
            .padding()
            .background(isEnabled ? Color.accentColor : Color.gray)
            .cornerRadius(12)
            .scaleEffect(configuration.isPressed ? 0.98 : 1.0)
            .animation(.easeInOut(duration: 0.1), value: configuration.isPressed)
    }
}

// ✅ 自定义 Layout (iOS 16+)
struct FlowLayout: Layout {
    var spacing: CGFloat = 8

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        let result = FlowResult(in: proposal.width ?? 0, subviews: subviews, spacing: spacing)
        return result.size
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        let result = FlowResult(in: bounds.width, subviews: subviews, spacing: spacing)
        for (index, subview) in subviews.enumerated() {
            subview.place(at: CGPoint(x: bounds.minX + result.positions[index].x,
                                      y: bounds.minY + result.positions[index].y),
                         proposal: .unspecified)
        }
    }
}
```

---

## 🌐 网络层

### 现代网络架构
```swift
// ✅ 网络服务协议
protocol NetworkService {
    func request<T: Decodable>(_ endpoint: Endpoint) async throws -> T
    func upload<T: Decodable>(_ endpoint: Endpoint, data: Data) async throws -> T
    func download(_ endpoint: Endpoint) async throws -> URL
}

// ✅ 端点定义
enum Endpoint {
    case getUsers(page: Int, limit: Int)
    case getUser(id: String)
    case createUser(CreateUserRequest)
    case updateUser(id: String, UpdateUserRequest)
    case deleteUser(id: String)

    var path: String {
        switch self {
        case .getUsers: return "/users"
        case .getUser(let id): return "/users/\(id)"
        case .createUser: return "/users"
        case .updateUser(let id, _): return "/users/\(id)"
        case .deleteUser(let id): return "/users/\(id)"
        }
    }

    var method: HTTPMethod {
        switch self {
        case .getUsers, .getUser: return .get
        case .createUser: return .post
        case .updateUser: return .put
        case .deleteUser: return .delete
        }
    }

    var body: Data? {
        switch self {
        case .createUser(let request):
            return try? JSONEncoder().encode(request)
        case .updateUser(_, let request):
            return try? JSONEncoder().encode(request)
        default:
            return nil
        }
    }
}

// ✅ 网络服务实现
final class URLSessionNetworkService: NetworkService {
    private let session: URLSession
    private let baseURL: URL
    private let decoder: JSONDecoder

    init(baseURL: URL, session: URLSession = .shared) {
        self.baseURL = baseURL
        self.session = session
        self.decoder = JSONDecoder()
        self.decoder.dateDecodingStrategy = .iso8601
    }

    func request<T: Decodable>(_ endpoint: Endpoint) async throws -> T {
        let request = try makeRequest(for: endpoint)
        let (data, response) = try await session.data(for: request)

        guard let httpResponse = response as? HTTPURLResponse else {
            throw NetworkError.invalidResponse
        }

        guard 200..<300 ~= httpResponse.statusCode else {
            throw NetworkError.httpError(statusCode: httpResponse.statusCode)
        }

        return try decoder.decode(T.self, from: data)
    }

    private func makeRequest(for endpoint: Endpoint) throws -> URLRequest {
        let url = baseURL.appendingPathComponent(endpoint.path)
        var request = URLRequest(url: url)
        request.httpMethod = endpoint.method.rawValue
        request.httpBody = endpoint.body
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        return request
    }
}
```

---

## 🗄️ 数据持久化

### Core Data 最佳实践
```swift
// ✅ 现代 Core Data 配置
final class CoreDataStack {
    static let shared = CoreDataStack()

    lazy var container: NSPersistentContainer = {
        let container = NSPersistentContainer(name: "MyApp")

        // CloudKit 同步
        container.persistentStoreDescriptions.first?.cloudKitContainerOptions =
            NSPersistentCloudKitContainerOptions(containerIdentifier: "iCloud.com.myapp")

        container.loadPersistentStores { _, error in
            if let error = error {
                fatalError("Failed to load Core Data: \(error)")
            }
        }

        container.viewContext.automaticallyMergesChangesFromParent = true
        container.viewContext.mergePolicy = NSMergeByPropertyObjectTrumpMergePolicy
        return container
    }()

    var viewContext: NSManagedObjectContext {
        container.viewContext
    }

    func newBackgroundContext() -> NSManagedObjectContext {
        container.newBackgroundContext()
    }

    func performBackgroundTask(_ block: @escaping (NSManagedObjectContext) -> Void) {
        container.performBackgroundTask(block)
    }
}

// ✅ @FetchRequest with SwiftUI
struct UsersListView: View {
    @Environment(\.managedObjectContext) private var viewContext

    @FetchRequest(
        sortDescriptors: [NSSortDescriptor(keyPath: \UserMO.createdAt, ascending: false)],
        animation: .default
    )
    private var users: FetchedResults<UserMO>

    var body: some View {
        List {
            ForEach(users) { user in
                UserRowView(user: user)
            }
            .onDelete(perform: deleteUsers)
        }
    }

    private func deleteUsers(at offsets: IndexSet) {
        withAnimation {
            offsets.map { users[$0] }.forEach(viewContext.delete)
            try? viewContext.save()
        }
    }
}
```

### SwiftData (iOS 17+)
```swift
// ✅ SwiftData 模型
@Model
final class User {
    var id: UUID
    var name: String
    var email: String
    @Relationship(deleteRule: .cascade) var posts: [Post]
    var createdAt: Date

    init(name: String, email: String) {
        self.id = UUID()
        self.name = name
        self.email = email
        self.posts = []
        self.createdAt = Date()
    }
}

// ✅ SwiftData 查询
struct UsersView: View {
    @Query(sort: \User.createdAt, order: .reverse)
    private var users: [User]

    @Environment(\.modelContext) private var modelContext

    var body: some View {
        List(users) { user in
            UserRow(user: user)
        }
    }
}
```

---

## 🔒 安全最佳实践

### Keychain 服务
```swift
// ✅ Keychain 封装
final class KeychainService {
    enum KeychainError: Error {
        case itemNotFound
        case duplicateItem
        case unexpectedStatus(OSStatus)
    }

    func save(_ data: Data, forKey key: String) throws {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrAccount as String: key,
            kSecValueData as String: data,
            kSecAttrAccessible as String: kSecAttrAccessibleWhenUnlockedThisDeviceOnly
        ]

        let status = SecItemAdd(query as CFDictionary, nil)

        guard status == errSecSuccess else {
            if status == errSecDuplicateItem {
                try update(data, forKey: key)
            } else {
                throw KeychainError.unexpectedStatus(status)
            }
            return
        }
    }

    func get(forKey key: String) throws -> Data {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrAccount as String: key,
            kSecReturnData as String: true,
            kSecMatchLimit as String: kSecMatchLimitOne
        ]

        var result: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &result)

        guard status == errSecSuccess, let data = result as? Data else {
            throw KeychainError.itemNotFound
        }

        return data
    }
}

// ✅ 生物识别认证
final class BiometricAuthService {
    private let context = LAContext()

    func authenticate() async throws -> Bool {
        var error: NSError?
        guard context.canEvaluatePolicy(.deviceOwnerAuthenticationWithBiometrics, error: &error) else {
            throw error ?? BiometricError.notAvailable
        }

        return try await context.evaluatePolicy(
            .deviceOwnerAuthenticationWithBiometrics,
            localizedReason: "Authenticate to access your data"
        )
    }
}
```

---

## ⚡ 性能优化

### 性能监控指标
| 指标 | 目标值 |
|------|--------|
| 冷启动时间 | < 400ms |
| 热启动时间 | < 200ms |
| 帧率 | 60/120 fps |
| 丢帧率 | < 1% |
| 崩溃率 | < 0.1% |
| 内存占用 | < 150MB |
| IPA 大小 | < 30MB |

### Instruments 优化
```swift
// ✅ 使用 os_signpost 标记
import os.signpost

let log = OSLog(subsystem: "com.myapp", category: "Performance")

func loadData() async {
    let signpostID = OSSignpostID(log: log)
    os_signpost(.begin, log: log, name: "Load Data", signpostID: signpostID)

    // 执行数据加载
    await fetchData()

    os_signpost(.end, log: log, name: "Load Data", signpostID: signpostID)
}

// ✅ 内存优化
final class ImageCache {
    private let cache = NSCache<NSString, UIImage>()

    init() {
        cache.countLimit = 100
        cache.totalCostLimit = 50 * 1024 * 1024 // 50MB

        NotificationCenter.default.addObserver(
            self,
            selector: #selector(clearCache),
            name: UIApplication.didReceiveMemoryWarningNotification,
            object: nil
        )
    }

    @objc private func clearCache() {
        cache.removeAllObjects()
    }
}
```

---

## 🧪 测试

### 单元测试
```swift
// ✅ ViewModel 测试
@MainActor
final class UserViewModelTests: XCTestCase {
    var sut: UserViewModel!
    var mockUseCase: MockGetUserUseCase!

    override func setUp() {
        super.setUp()
        mockUseCase = MockGetUserUseCase()
        sut = UserViewModel(getUserUseCase: mockUseCase)
    }

    func test_loadUser_success_updatesState() async {
        // Given
        let expectedUser = User(id: "1", name: "Test")
        mockUseCase.result = .success(expectedUser)

        // When
        sut.loadUser(id: "1")
        await Task.yield() // 等待异步操作

        // Then
        if case .loaded(let user) = sut.state {
            XCTAssertEqual(user.id, expectedUser.id)
        } else {
            XCTFail("Expected loaded state")
        }
    }
}

// ✅ 网络测试
final class NetworkServiceTests: XCTestCase {
    var sut: URLSessionNetworkService!
    var mockSession: URLSession!

    override func setUp() {
        let config = URLSessionConfiguration.ephemeral
        config.protocolClasses = [MockURLProtocol.self]
        mockSession = URLSession(configuration: config)
        sut = URLSessionNetworkService(baseURL: URL(string: "https://api.example.com")!, session: mockSession)
    }

    func test_request_success_decodesResponse() async throws {
        // Given
        let expectedData = """
        {"id": "1", "name": "Test"}
        """.data(using: .utf8)!

        MockURLProtocol.requestHandler = { request in
            let response = HTTPURLResponse(url: request.url!, statusCode: 200, httpVersion: nil, headerFields: nil)!
            return (response, expectedData)
        }

        // When
        let user: User = try await sut.request(.getUser(id: "1"))

        // Then
        XCTAssertEqual(user.id, "1")
        XCTAssertEqual(user.name, "Test")
    }
}
```

### UI 测试
```swift
// ✅ SwiftUI Preview 测试
struct UserRowView_Previews: PreviewProvider {
    static var previews: some View {
        Group {
            UserRowView(user: .mock)
                .previewDisplayName("Default")

            UserRowView(user: .mockLongName)
                .previewDisplayName("Long Name")

            UserRowView(user: .mock)
                .preferredColorScheme(.dark)
                .previewDisplayName("Dark Mode")
        }
        .previewLayout(.sizeThatFits)
    }
}

// ✅ XCTest UI 测试
final class LoginUITests: XCTestCase {
    var app: XCUIApplication!

    override func setUp() {
        continueAfterFailure = false
        app = XCUIApplication()
        app.launchArguments = ["--uitesting"]
        app.launch()
    }

    func test_login_withValidCredentials_showsHomeScreen() {
        let emailField = app.textFields["email"]
        let passwordField = app.secureTextFields["password"]
        let loginButton = app.buttons["login"]

        emailField.tap()
        emailField.typeText("test@example.com")

        passwordField.tap()
        passwordField.typeText("password123")

        loginButton.tap()

        XCTAssertTrue(app.tabBars["MainTabBar"].waitForExistence(timeout: 5))
    }
}
```

---

## 📋 iOS 开发检查清单

### 架构
- [ ] Clean Architecture + MVVM-C
- [ ] SPM 模块化
- [ ] 依赖注入
- [ ] Coordinator 导航

### 代码质量
- [ ] Swift Concurrency
- [ ] Protocol-Oriented
- [ ] Value Types 优先
- [ ] 避免循环引用

### 性能
- [ ] 冷启动 < 400ms
- [ ] 60/120 fps 流畅
- [ ] Instruments 分析
- [ ] 内存优化

### 安全
- [ ] Keychain 敏感数据
- [ ] 生物识别
- [ ] App Transport Security
- [ ] 数据加密

### 测试
- [ ] ViewModel 单元测试
- [ ] 网络层测试
- [ ] UI 测试
- [ ] Snapshot 测试

---

**iOS 开发原则总结**：
Clean Architecture、Swift Concurrency、SwiftUI 声明式 UI、Protocol-Oriented、Value Types、Combine 响应式、Core Data/SwiftData 持久化、Keychain 安全、Instruments 性能分析、XCTest 测试覆盖
