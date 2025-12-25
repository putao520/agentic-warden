# Android 应用开发规范 - CODING-STANDARDS-ANDROID

**版本**: 2.0.0
**适用范围**: Android 原生应用开发（Kotlin/Java、Jetpack、NDK）
**技术栈**: Kotlin、Jetpack Compose、Coroutines、Hilt、Room、Retrofit
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
       - 禁止在旧 Activity/Fragment 上添加新功能
       - 禁止保留废弃的 XML 布局和资源

铁律4: Context7 调研先行
       - 使用 Jetpack 官方库和成熟第三方库
       - 禁止自己实现网络、图片加载等基础设施
```

---

## 🏗️ 项目架构

### Clean Architecture + MVVM
```
app/
├── data/                    # 数据层
│   ├── local/               # 本地数据源
│   │   ├── dao/             # Room DAO
│   │   └── entity/          # 数据库实体
│   ├── remote/              # 远程数据源
│   │   ├── api/             # Retrofit API
│   │   └── dto/             # 数据传输对象
│   └── repository/          # Repository 实现
├── domain/                  # 领域层
│   ├── model/               # 领域模型
│   ├── repository/          # Repository 接口
│   └── usecase/             # 用例
├── presentation/            # 表现层
│   ├── ui/                  # Compose UI
│   │   ├── screens/         # 页面
│   │   ├── components/      # 可复用组件
│   │   └── theme/           # 主题
│   └── viewmodel/           # ViewModel
└── di/                      # 依赖注入模块
```

### 模块化架构
```kotlin
// ✅ 多模块项目结构
// settings.gradle.kts
include(":app")
include(":core:common")
include(":core:network")
include(":core:database")
include(":core:ui")
include(":feature:home")
include(":feature:profile")
include(":feature:settings")

// ✅ 模块间依赖规则
// feature 模块只能依赖 core 模块
// app 模块依赖所有 feature 模块
// core 模块之间尽量独立
```

---

## 📜 Kotlin 编码规范

### Coroutines 最佳实践
```kotlin
// ✅ ViewModel 中使用 viewModelScope
class UserViewModel @Inject constructor(
    private val getUserUseCase: GetUserUseCase
) : ViewModel() {

    private val _uiState = MutableStateFlow<UserUiState>(UserUiState.Loading)
    val uiState: StateFlow<UserUiState> = _uiState.asStateFlow()

    fun loadUser(userId: String) {
        viewModelScope.launch {
            _uiState.value = UserUiState.Loading
            getUserUseCase(userId)
                .catch { e -> _uiState.value = UserUiState.Error(e.message) }
                .collect { user -> _uiState.value = UserUiState.Success(user) }
        }
    }
}

// ✅ Repository 中使用 Dispatchers
class UserRepositoryImpl @Inject constructor(
    private val api: UserApi,
    private val dao: UserDao,
    @IoDispatcher private val ioDispatcher: CoroutineDispatcher
) : UserRepository {

    override fun getUser(id: String): Flow<User> = flow {
        // 先从本地获取
        dao.getUser(id)?.let { emit(it.toDomain()) }

        // 再从网络更新
        val remoteUser = api.getUser(id)
        dao.insertUser(remoteUser.toEntity())
        emit(remoteUser.toDomain())
    }.flowOn(ioDispatcher)
}

// ✅ 结构化并发
suspend fun fetchDataConcurrently() = coroutineScope {
    val users = async { userRepository.getUsers() }
    val posts = async { postRepository.getPosts() }
    CombinedData(users.await(), posts.await())
}
```

### Flow 高级用法
```kotlin
// ✅ StateFlow + SharedFlow
class SearchViewModel @Inject constructor(
    private val searchUseCase: SearchUseCase
) : ViewModel() {

    private val searchQuery = MutableStateFlow("")

    val searchResults: StateFlow<List<SearchResult>> = searchQuery
        .debounce(300)
        .filter { it.length >= 2 }
        .distinctUntilChanged()
        .flatMapLatest { query ->
            searchUseCase(query)
                .catch { emit(emptyList()) }
        }
        .stateIn(
            scope = viewModelScope,
            started = SharingStarted.WhileSubscribed(5000),
            initialValue = emptyList()
        )

    fun onQueryChanged(query: String) {
        searchQuery.value = query
    }
}

// ✅ Channel 用于一次性事件
private val _events = Channel<UiEvent>(Channel.BUFFERED)
val events: Flow<UiEvent> = _events.receiveAsFlow()

fun showSnackbar(message: String) {
    viewModelScope.launch {
        _events.send(UiEvent.ShowSnackbar(message))
    }
}
```

---

## 🎨 Jetpack Compose

### Compose 架构
```kotlin
// ✅ 状态提升 + 单向数据流
@Composable
fun UserScreen(
    viewModel: UserViewModel = hiltViewModel()
) {
    val uiState by viewModel.uiState.collectAsStateWithLifecycle()

    UserContent(
        uiState = uiState,
        onRefresh = viewModel::refresh,
        onItemClick = viewModel::onItemClick
    )
}

// ✅ 无状态 Composable（可测试）
@Composable
fun UserContent(
    uiState: UserUiState,
    onRefresh: () -> Unit,
    onItemClick: (User) -> Unit,
    modifier: Modifier = Modifier
) {
    when (uiState) {
        is UserUiState.Loading -> LoadingIndicator()
        is UserUiState.Success -> UserList(
            users = uiState.users,
            onItemClick = onItemClick
        )
        is UserUiState.Error -> ErrorMessage(
            message = uiState.message,
            onRetry = onRefresh
        )
    }
}
```

### 性能优化
```kotlin
// ✅ 使用 remember 和 derivedStateOf
@Composable
fun ItemList(items: List<Item>) {
    val sortedItems by remember(items) {
        derivedStateOf { items.sortedBy { it.name } }
    }

    LazyColumn {
        items(
            items = sortedItems,
            key = { it.id }  // ✅ 使用 key 优化重组
        ) { item ->
            ItemRow(item = item)
        }
    }
}

// ✅ 使用 Immutable 标注
@Immutable
data class UserUiModel(
    val id: String,
    val name: String,
    val avatarUrl: String
)

// ✅ 使用 Stable 标注回调
@Stable
class UserListCallbacks(
    val onItemClick: (User) -> Unit,
    val onItemLongClick: (User) -> Unit,
    val onDeleteClick: (User) -> Unit
)
```

### 自定义 Compose 组件
```kotlin
// ✅ 自定义 Modifier
fun Modifier.shimmerEffect(): Modifier = composed {
    var size by remember { mutableStateOf(IntSize.Zero) }
    val transition = rememberInfiniteTransition(label = "shimmer")
    val startOffsetX by transition.animateFloat(
        initialValue = -2 * size.width.toFloat(),
        targetValue = 2 * size.width.toFloat(),
        animationSpec = infiniteRepeatable(
            animation = tween(1000)
        ),
        label = "shimmerOffset"
    )

    background(
        brush = Brush.linearGradient(
            colors = listOf(
                Color.LightGray.copy(alpha = 0.6f),
                Color.LightGray.copy(alpha = 0.2f),
                Color.LightGray.copy(alpha = 0.6f)
            ),
            start = Offset(startOffsetX, 0f),
            end = Offset(startOffsetX + size.width, size.height.toFloat())
        )
    ).onGloballyPositioned { size = it.size }
}

// ✅ 自定义 Layout
@Composable
fun FlowRow(
    modifier: Modifier = Modifier,
    horizontalSpacing: Dp = 8.dp,
    verticalSpacing: Dp = 8.dp,
    content: @Composable () -> Unit
) {
    Layout(
        content = content,
        modifier = modifier
    ) { measurables, constraints ->
        // 测量和布局逻辑
        layout(width, height) {
            // 放置子元素
        }
    }
}
```

---

## 💉 依赖注入 (Hilt)

### Hilt 模块组织
```kotlin
// ✅ Network 模块
@Module
@InstallIn(SingletonComponent::class)
object NetworkModule {

    @Provides
    @Singleton
    fun provideOkHttpClient(
        authInterceptor: AuthInterceptor,
        loggingInterceptor: HttpLoggingInterceptor
    ): OkHttpClient = OkHttpClient.Builder()
        .addInterceptor(authInterceptor)
        .addInterceptor(loggingInterceptor)
        .connectTimeout(30, TimeUnit.SECONDS)
        .readTimeout(30, TimeUnit.SECONDS)
        .build()

    @Provides
    @Singleton
    fun provideRetrofit(
        okHttpClient: OkHttpClient,
        json: Json
    ): Retrofit = Retrofit.Builder()
        .baseUrl(BuildConfig.API_BASE_URL)
        .client(okHttpClient)
        .addConverterFactory(json.asConverterFactory("application/json".toMediaType()))
        .build()
}

// ✅ Dispatcher 模块
@Module
@InstallIn(SingletonComponent::class)
object DispatcherModule {

    @IoDispatcher
    @Provides
    fun provideIoDispatcher(): CoroutineDispatcher = Dispatchers.IO

    @DefaultDispatcher
    @Provides
    fun provideDefaultDispatcher(): CoroutineDispatcher = Dispatchers.Default

    @MainDispatcher
    @Provides
    fun provideMainDispatcher(): CoroutineDispatcher = Dispatchers.Main
}

@Qualifier
@Retention(AnnotationRetention.BINARY)
annotation class IoDispatcher

@Qualifier
@Retention(AnnotationRetention.BINARY)
annotation class DefaultDispatcher

@Qualifier
@Retention(AnnotationRetention.BINARY)
annotation class MainDispatcher
```

---

## 🗄️ 数据持久化 (Room)

### Room 最佳实践
```kotlin
// ✅ Entity 设计
@Entity(
    tableName = "users",
    indices = [Index(value = ["email"], unique = true)]
)
data class UserEntity(
    @PrimaryKey
    val id: String,
    val name: String,
    val email: String,
    @ColumnInfo(name = "avatar_url")
    val avatarUrl: String?,
    @ColumnInfo(name = "created_at")
    val createdAt: Long = System.currentTimeMillis()
)

// ✅ DAO with Flow
@Dao
interface UserDao {
    @Query("SELECT * FROM users ORDER BY created_at DESC")
    fun observeUsers(): Flow<List<UserEntity>>

    @Query("SELECT * FROM users WHERE id = :id")
    suspend fun getUser(id: String): UserEntity?

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun insertUsers(users: List<UserEntity>)

    @Transaction
    suspend fun replaceAllUsers(users: List<UserEntity>) {
        deleteAll()
        insertUsers(users)
    }

    @Query("DELETE FROM users")
    suspend fun deleteAll()
}

// ✅ Database with TypeConverter
@Database(
    entities = [UserEntity::class, PostEntity::class],
    version = 1,
    exportSchema = true
)
@TypeConverters(Converters::class)
abstract class AppDatabase : RoomDatabase() {
    abstract fun userDao(): UserDao
    abstract fun postDao(): PostDao
}

class Converters {
    @TypeConverter
    fun fromTimestamp(value: Long?): Date? = value?.let { Date(it) }

    @TypeConverter
    fun dateToTimestamp(date: Date?): Long? = date?.time
}
```

---

## 🌐 网络层 (Retrofit)

### Retrofit 配置
```kotlin
// ✅ API 接口定义
interface UserApi {
    @GET("users")
    suspend fun getUsers(
        @Query("page") page: Int,
        @Query("limit") limit: Int = 20
    ): Response<List<UserDto>>

    @GET("users/{id}")
    suspend fun getUser(@Path("id") id: String): Response<UserDto>

    @POST("users")
    suspend fun createUser(@Body user: CreateUserRequest): Response<UserDto>

    @Multipart
    @POST("users/{id}/avatar")
    suspend fun uploadAvatar(
        @Path("id") id: String,
        @Part image: MultipartBody.Part
    ): Response<AvatarResponse>
}

// ✅ 网络结果封装
sealed class NetworkResult<out T> {
    data class Success<T>(val data: T) : NetworkResult<T>()
    data class Error(val code: Int, val message: String) : NetworkResult<Nothing>()
    data class Exception(val throwable: Throwable) : NetworkResult<Nothing>()
}

// ✅ 安全 API 调用
suspend fun <T> safeApiCall(
    apiCall: suspend () -> Response<T>
): NetworkResult<T> = try {
    val response = apiCall()
    if (response.isSuccessful) {
        response.body()?.let { NetworkResult.Success(it) }
            ?: NetworkResult.Error(response.code(), "Empty body")
    } else {
        NetworkResult.Error(response.code(), response.message())
    }
} catch (e: IOException) {
    NetworkResult.Exception(e)
} catch (e: HttpException) {
    NetworkResult.Error(e.code(), e.message())
}
```

---

## 🔒 安全最佳实践

### 数据加密
```kotlin
// ✅ EncryptedSharedPreferences
@Provides
@Singleton
fun provideEncryptedPrefs(@ApplicationContext context: Context): SharedPreferences {
    val masterKey = MasterKey.Builder(context)
        .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
        .build()

    return EncryptedSharedPreferences.create(
        context,
        "secure_prefs",
        masterKey,
        EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
        EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
    )
}

// ✅ Keystore 密钥管理
class KeystoreManager {
    private val keyStore = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }

    fun getOrCreateSecretKey(alias: String): SecretKey {
        keyStore.getEntry(alias, null)?.let {
            return (it as KeyStore.SecretKeyEntry).secretKey
        }

        val keyGenerator = KeyGenerator.getInstance(
            KeyProperties.KEY_ALGORITHM_AES,
            "AndroidKeyStore"
        )
        keyGenerator.init(
            KeyGenParameterSpec.Builder(
                alias,
                KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT
            )
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setUserAuthenticationRequired(false)
                .build()
        )
        return keyGenerator.generateKey()
    }
}
```

### ProGuard/R8 配置
```proguard
# ✅ 必须保留的规则
-keepattributes *Annotation*
-keepattributes Signature
-keepattributes SourceFile,LineNumberTable

# Kotlin Serialization
-keepattributes InnerClasses
-keep class kotlinx.serialization.** { *; }
-keepclassmembers class * {
    @kotlinx.serialization.SerialName <fields>;
}

# Retrofit
-keepclassmembers,allowshrinking,allowobfuscation interface * {
    @retrofit2.http.* <methods>;
}

# Room
-keep class * extends androidx.room.RoomDatabase
-keep @androidx.room.Entity class *
```

---

## ⚡ 性能优化

### 启动优化
```kotlin
// ✅ App Startup 库
class MyInitializer : Initializer<MyDependency> {
    override fun create(context: Context): MyDependency {
        // 延迟初始化非关键依赖
        return MyDependency()
    }

    override fun dependencies(): List<Class<out Initializer<*>>> {
        return listOf(WorkManagerInitializer::class.java)
    }
}

// ✅ Baseline Profiles
// 使用 Macrobenchmark 生成 Baseline Profile
@OptIn(ExperimentalBaselineProfilesApi::class)
@RunWith(AndroidJUnit4::class)
class BaselineProfileGenerator {
    @get:Rule
    val baselineProfileRule = BaselineProfileRule()

    @Test
    fun generate() = baselineProfileRule.collect(packageName = "com.example.app") {
        startActivityAndWait()
        // 执行关键用户路径
    }
}
```

### 内存优化
```kotlin
// ✅ 避免内存泄漏
class MyFragment : Fragment() {
    private var _binding: FragmentMyBinding? = null
    private val binding get() = _binding!!

    override fun onDestroyView() {
        super.onDestroyView()
        _binding = null  // 必须清空
    }
}

// ✅ WeakReference 回调
class NetworkCallback(fragment: MyFragment) {
    private val fragmentRef = WeakReference(fragment)

    fun onSuccess(data: Data) {
        fragmentRef.get()?.handleSuccess(data)
    }
}

// ✅ LeakCanary 集成
debugImplementation("com.squareup.leakcanary:leakcanary-android:2.12")
```

### 性能监控指标
| 指标 | 目标值 |
|------|--------|
| 冷启动时间 | < 2s |
| 热启动时间 | < 500ms |
| 帧率 | 60 fps |
| 丢帧率 | < 1% |
| ANR 率 | < 0.1% |
| 崩溃率 | < 0.1% |
| 内存占用 | < 200MB |
| APK 大小 | < 50MB |

---

## 🧪 测试

### 单元测试
```kotlin
// ✅ ViewModel 测试
@OptIn(ExperimentalCoroutinesApi::class)
class UserViewModelTest {
    @get:Rule
    val mainDispatcherRule = MainDispatcherRule()

    private lateinit var viewModel: UserViewModel
    private val getUserUseCase: GetUserUseCase = mockk()

    @Before
    fun setup() {
        viewModel = UserViewModel(getUserUseCase)
    }

    @Test
    fun `loadUser success updates state`() = runTest {
        val user = User("1", "Test User")
        coEvery { getUserUseCase("1") } returns flowOf(user)

        viewModel.loadUser("1")

        assertEquals(UserUiState.Success(user), viewModel.uiState.value)
    }
}

// ✅ Repository 测试
@OptIn(ExperimentalCoroutinesApi::class)
class UserRepositoryTest {
    private lateinit var repository: UserRepository
    private val api: UserApi = mockk()
    private val dao: UserDao = mockk()

    @Test
    fun `getUser returns cached then remote`() = runTest {
        val cachedUser = UserEntity("1", "Cached")
        val remoteUser = UserDto("1", "Remote")

        coEvery { dao.getUser("1") } returns cachedUser
        coEvery { api.getUser("1") } returns Response.success(remoteUser)
        coEvery { dao.insertUser(any()) } just Runs

        val results = repository.getUser("1").toList()

        assertEquals(2, results.size)
        assertEquals("Cached", results[0].name)
        assertEquals("Remote", results[1].name)
    }
}
```

### UI 测试 (Compose)
```kotlin
// ✅ Compose UI 测试
class UserScreenTest {
    @get:Rule
    val composeTestRule = createComposeRule()

    @Test
    fun `shows loading indicator when loading`() {
        composeTestRule.setContent {
            UserContent(uiState = UserUiState.Loading, onRefresh = {}, onItemClick = {})
        }

        composeTestRule
            .onNodeWithContentDescription("Loading")
            .assertIsDisplayed()
    }

    @Test
    fun `shows user list when success`() {
        val users = listOf(User("1", "Test User"))

        composeTestRule.setContent {
            UserContent(
                uiState = UserUiState.Success(users),
                onRefresh = {},
                onItemClick = {}
            )
        }

        composeTestRule
            .onNodeWithText("Test User")
            .assertIsDisplayed()
    }
}
```

---

## 📋 Android 开发检查清单

### 架构
- [ ] Clean Architecture + MVVM
- [ ] 模块化项目结构
- [ ] 依赖注入 (Hilt)
- [ ] 单向数据流

### 代码质量
- [ ] Kotlin Coroutines + Flow
- [ ] 状态提升 Compose
- [ ] 使用 Immutable 数据类
- [ ] 避免内存泄漏

### 性能
- [ ] 冷启动 < 2s
- [ ] 60 fps 流畅
- [ ] Baseline Profiles
- [ ] R8 代码混淆

### 安全
- [ ] EncryptedSharedPreferences
- [ ] Keystore 密钥管理
- [ ] ProGuard 规则
- [ ] 证书固定 (Certificate Pinning)

### 测试
- [ ] ViewModel 单元测试
- [ ] Repository 测试
- [ ] Compose UI 测试
- [ ] 端到端测试

---

**Android 开发原则总结**：
Clean Architecture、Kotlin Coroutines、Jetpack Compose、Hilt依赖注入、Room数据库、Retrofit网络、安全存储、性能优化、单元测试、模块化架构
