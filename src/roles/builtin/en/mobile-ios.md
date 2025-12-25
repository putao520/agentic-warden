# iOS Application Development Standards - CODING-STANDARDS-IOS

**Version**: 2.0.0
**Scope**: iOS/iPadOS/watchOS/tvOS native application development
**Tech Stack**: Swift, SwiftUI, UIKit, Combine, Core Data, URLSession
**Last Updated**: 2025-12-25

---

## 🚨 Core Iron Rules (Inherited from common.md)

> **Must follow the four core iron rules from common.md**

```
Iron Rule 1: SPEC is the Single Source of Truth (SSOT)
       - UI/UX implementation must comply with SPEC definitions
       - Data models, API interfaces based on SPEC

Iron Rule 2: Smart Reuse and Destroy-Rebuild
       - Existing component fully matches → Direct reuse
       - Partial match → Delete and rebuild, no incremental modifications

Iron Rule 3: Prohibit Incremental Development
       - Prohibit adding new features to old ViewController
       - Prohibit retaining deprecated Storyboard and XIB

Iron Rule 4: Context7 Research First
       - Use Apple official frameworks and mature third-party libraries
       - Prohibit implementing infrastructure like networking and image caching yourself
```

---

## 🏗️ Project Architecture

### Clean Architecture + MVVM-C
```
MyApp/
├── Application/             # App lifecycle
│   ├── AppDelegate.swift
│   ├── SceneDelegate.swift
│   └── AppCoordinator.swift
├── Domain/                  # Domain layer (pure Swift)
│   ├── Entities/            # Domain entities
│   ├── UseCases/            # Use case interfaces
│   └── Repositories/        # Repository protocols
├── Data/                    # Data layer
│   ├── Network/             # Network layer
│   │   ├── API/             # API definitions
│   │   ├── DTOs/            # Data transfer objects
│   │   └── NetworkService.swift
│   ├── Persistence/         # Persistence
│   │   ├── CoreData/        # Core Data models
│   │   └── UserDefaults/    # UserDefaults wrapper
│   └── Repositories/        # Repository implementations
├── Presentation/            # Presentation layer
│   ├── Scenes/              # Screen modules
│   │   ├── Home/
│   │   ├── Profile/
│   │   └── Settings/
│   ├── Common/              # Shared UI
│   │   ├── Views/
│   │   ├── ViewModifiers/
│   │   └── Components/
│   └── Coordinators/        # Navigation coordinators
├── Core/                    # Core utilities
│   ├── Extensions/
│   ├── Utilities/
│   └── Constants/
└── Resources/               # Resource files
    ├── Assets.xcassets
    ├── Localizable.strings
    └── Info.plist
```

### Modular Architecture (SPM)
```swift
// ✅ Package.swift multi-module configuration
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

## 📜 Swift Coding Standards

### Protocol-Oriented Programming
```swift
// ✅ Use protocols to define abstractions
protocol UserRepository {
    func getUser(id: String) async throws -> User
    func saveUser(_ user: User) async throws
    func observeUser(id: String) -> AsyncStream<User>
}

// ✅ Protocol extensions provide default implementations
extension UserRepository {
    func getUserOrNil(id: String) async -> User? {
        try? await getUser(id: id)
    }
}

// ✅ Protocol composition
typealias DataRepository = UserRepository & PostRepository & CommentRepository

// ✅ Use some keyword to hide concrete types
func makeUserRepository() -> some UserRepository {
    UserRepositoryImpl(networkService: NetworkService.shared)
}
```

### Modern Concurrency (Swift Concurrency)
```swift
// ✅ Actor protects shared state
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

// ✅ TaskGroup concurrent execution
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

// ✅ AsyncStream async sequence
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

// ✅ Structured concurrency with cancellation
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

### Value Types First
```swift
// ✅ Use struct instead of class
struct User: Identifiable, Codable, Hashable {
    let id: String
    var name: String
    var email: String
    var avatarURL: URL?
    var createdAt: Date

    // Use CodingKeys for custom mapping
    enum CodingKeys: String, CodingKey {
        case id
        case name
        case email
        case avatarURL = "avatar_url"
        case createdAt = "created_at"
    }
}

// ✅ Copy-on-Write semantics
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

### Architecture Patterns
```swift
// ✅ State management
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

// ✅ View organization
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

// ✅ Stateless view (testable)
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

### Performance Optimization
```swift
// ✅ Use @ViewBuilder for conditional rendering
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

// ✅ Use EquatableView to optimize redraws
struct ExpensiveView: View, Equatable {
    let data: ExpensiveData

    var body: some View {
        // Complex rendering
    }

    static func == (lhs: ExpensiveView, rhs: ExpensiveView) -> Bool {
        lhs.data.id == rhs.data.id
    }
}

// ✅ Use LazyVStack/LazyHStack
struct ItemListView: View {
    let items: [Item]

    var body: some View {
        ScrollView {
            LazyVStack(spacing: 16) {
                ForEach(items) { item in
                    ItemRow(item: item)
                        .id(item.id)  // Optimize diff
                }
            }
        }
    }
}

// ✅ Custom PreferenceKey
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

### Custom Components
```swift
// ✅ Custom ViewModifier
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

// ✅ Custom ButtonStyle
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

// ✅ Custom Layout (iOS 16+)
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

## 🌐 Network Layer

### Modern Network Architecture
```swift
// ✅ Network service protocol
protocol NetworkService {
    func request<T: Decodable>(_ endpoint: Endpoint) async throws -> T
    func upload<T: Decodable>(_ endpoint: Endpoint, data: Data) async throws -> T
    func download(_ endpoint: Endpoint) async throws -> URL
}

// ✅ Endpoint definition
enum Endpoint {
    case getUsers(page: Int, limit: Int)
    case getUser(id: String)
    case CreateUser(CreateUserRequest)
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

// ✅ Network service implementation
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

## 🗄️ Data Persistence

### Core Data Best Practices
```swift
// ✅ Modern Core Data configuration
final class CoreDataStack {
    static let shared = CoreDataStack()

    lazy var container: NSPersistentContainer = {
        let container = NSPersistentContainer(name: "MyApp")

        // CloudKit sync
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
// ✅ SwiftData model
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

// ✅ SwiftData query
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

## 🔒 Security Best Practices

### Keychain Services
```swift
// ✅ Keychain wrapper
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

// ✅ Biometric authentication
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

## ⚡ Performance Optimization

### Performance Monitoring Metrics
| Metric | Target |
|--------|--------|
| Cold Start Time | < 400ms |
| Warm Start Time | < 200ms |
| Frame Rate | 60/120 fps |
| Frame Drop Rate | < 1% |
| Crash Rate | < 0.1% |
| Memory Usage | < 150MB |
| IPA Size | < 30MB |

### Instruments Optimization
```swift
// ✅ Use os_signpost for marking
import os.signpost

let log = OSLog(subsystem: "com.myapp", category: "Performance")

func loadData() async {
    let signpostID = OSSignpostID(log: log)
    os_signpost(.begin, log: log, name: "Load Data", signpostID: signpostID)

    // Execute data loading
    await fetchData()

    os_signpost(.end, log: log, name: "Load Data", signpostID: signpostID)
}

// ✅ Memory optimization
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

## 🧪 Testing

### Unit Tests
```swift
// ✅ ViewModel testing
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
        await Task.yield() // Wait for async operation

        // Then
        if case .loaded(let user) = sut.state {
            XCTAssertEqual(user.id, expectedUser.id)
        } else {
            XCTFail("Expected loaded state")
        }
    }
}

// ✅ Network testing
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

### UI Testing
```swift
// ✅ SwiftUI Preview testing
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

// ✅ XCTest UI testing
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

## 📋 iOS Development Checklist

### Architecture
- [ ] Clean Architecture + MVVM-C
- [ ] SPM modularization
- [ ] Dependency injection
- [ ] Coordinator navigation

### Code Quality
- [ ] Swift Concurrency
- [ ] Protocol-Oriented
- [ ] Value types first
- [ ] Avoid circular references

### Performance
- [ ] Cold start < 400ms
- [ ] 60/120 fps smooth
- [ ] Instruments analysis
- [ ] Memory optimization

### Security
- [ ] Keychain sensitive data
- [ ] Biometric authentication
- [ ] App Transport Security
- [ ] Data encryption

### Testing
- [ ] ViewModel unit tests
- [ ] Network layer tests
- [ ] UI tests
- [ ] Snapshot tests

---

**iOS Development Principles Summary**:
Clean Architecture, Swift Concurrency, SwiftUI Declarative UI, Protocol-Oriented, Value Types, Combine Reactive, Core Data/SwiftData Persistence, Keychain Security, Instruments Performance Analysis, XCTest Test Coverage
