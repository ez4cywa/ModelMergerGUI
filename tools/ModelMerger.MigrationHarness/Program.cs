using ModelMerger.Core.Merging;
using ModelMerger.Core.Preview;
using PhilLibX;
using PhilLibX.Mathematics;
using System.Buffers.Binary;
using System.Diagnostics;
using System.Security.Cryptography;
using System.Text.Json;
using System.Text.Json.Nodes;

if (args.Length == 0)
{
    return Usage();
}

switch (args[0].ToLowerInvariant())
{
    case "generate":
        if (args.Length != 4 ||
            !int.TryParse(args[2], out var partCount) ||
            !int.TryParse(args[3], out var trianglesPerPart) ||
            partCount < 2 ||
            trianglesPerPart < 1)
        {
            return Usage();
        }

        Generate(args[1], partCount, trianglesPerPart);
        return 0;

    case "summarize":
        if (args.Length != 2)
        {
            return Usage();
        }

        Console.WriteLine(JsonSerializer.Serialize(
            Summarize(args[1]),
            new JsonSerializerOptions { WriteIndented = true }));
        return 0;

    case "benchmark":
        if (args.Length != 5 ||
            !int.TryParse(args[2], out var benchmarkPartCount) ||
            !int.TryParse(args[3], out var iterations) ||
            iterations < 1)
        {
            return Usage();
        }

        await BenchmarkAsync(
            new ModelMergeService(),
            "csharp",
            args[1],
            benchmarkPartCount,
            iterations,
            args[4]);
        return 0;

    case "benchmark-rust":
        if (args.Length != 6 ||
            !int.TryParse(args[3], out var rustBenchmarkPartCount) ||
            !int.TryParse(args[4], out var rustIterations) ||
            rustIterations < 1)
        {
            return Usage();
        }

        await BenchmarkAsync(
            new RustWorkerMergeService(args[1]),
            "rust-worker",
            args[2],
            rustBenchmarkPartCount,
            rustIterations,
            args[5]);
        return 0;

    case "compare":
        if (args.Length != 3)
        {
            return Usage();
        }

        return Compare(args[1], args[2]);

    case "compare-preview":
        if (args.Length != 5 ||
            !int.TryParse(args[3], out var previewTriangleLimit) ||
            !int.TryParse(args[4], out var previewIterations) ||
            previewTriangleLimit < 1 ||
            previewIterations < 1)
        {
            return Usage();
        }

        return await ComparePreviewAsync(
            args[1],
            args[2],
            previewTriangleLimit,
            previewIterations);

    case "benchmark-preview-rust":
        if (args.Length != 5 ||
            !int.TryParse(args[3], out var rustPreviewTriangleLimit) ||
            !int.TryParse(args[4], out var rustPreviewIterations) ||
            rustPreviewTriangleLimit < 1 ||
            rustPreviewIterations < 1)
        {
            return Usage();
        }

        await BenchmarkPreviewAsync(
            new RustWorkerModelPreviewService(args[1]),
            "rust-worker",
            args[2],
            rustPreviewTriangleLimit,
            rustPreviewIterations);
        return 0;

    default:
        return Usage();
}

static int Usage()
{
    Console.Error.WriteLine(
        """
        Usage:
          generate <output-directory> <part-count> <triangles-per-part>
          summarize <cast-file>
          benchmark <input-directory> <part-count> <iterations> <output-directory>
          benchmark-rust <worker-exe> <input-directory> <part-count> <iterations> <output-directory>
          compare <left-cast-file> <right-cast-file>
          compare-preview <worker-exe> <cast-file> <triangle-limit> <iterations>
          benchmark-preview-rust <worker-exe> <cast-file> <triangle-limit> <iterations>
        """);
    return 2;
}

static void Generate(string outputDirectory, int partCount, int trianglesPerPart)
{
    Directory.CreateDirectory(outputDirectory);
    for (var partIndex = 0; partIndex < partCount; partIndex++)
    {
        var path = Path.Combine(outputDirectory, $"part-{partIndex:D2}.cast");
        CreatePart(partIndex, partCount, trianglesPerPart).Save(path);
        Console.WriteLine(path);
    }
}

static Model CreatePart(int partIndex, int partCount, int trianglesPerPart)
{
    var model = new Model($"part-{partIndex:D2}");
    if (partIndex == 0)
    {
        model.Bones.Add(new Model.Bone("root"));
    }
    else
    {
        model.Bones.Add(new Model.Bone($"attach-{partIndex:D2}"));
    }

    if (partIndex + 1 < partCount)
    {
        model.Bones.Add(new Model.Bone(
            $"attach-{partIndex + 1:D2}",
            0,
            new Vector3(0, 0, 1),
            IdentityRotation()));
    }

    var material = new Model.Material(partIndex % 2 == 0 ? "shared-material" : $"material-{partIndex:D2}");
    material.Images[material.DiffuseMapName] = $"textures/part-{partIndex:D2}-albedo.png";
    model.Materials.Add(material);
    model.Shapes.Add($"shape-{partIndex:D2}");

    var mesh = new Model.Mesh(trianglesPerPart * 3, trianglesPerPart);
    mesh.MaterialIndices.Add(0);
    for (var triangle = 0; triangle < trianglesPerPart; triangle++)
    {
        var baseIndex = mesh.Vertices.Count;
        var x = triangle % 128;
        var y = triangle / 128;
        AddVertex(x, y, partIndex);
        AddVertex(x + 0.75f, y, partIndex);
        AddVertex(x, y + 0.75f, partIndex);
        mesh.Faces.Add(new Model.Face(baseIndex, baseIndex + 1, baseIndex + 2));
    }

    if (mesh.Vertices.Count > 0)
    {
        mesh.Vertices[0].Shapes.Add(new Model.Vertex.Shape(0, new Vector3(0.1f, 0.2f, 0.3f)));
    }

    model.Meshes.Add(mesh);
    model.GenerateGlobalBoneData();
    return model;

    void AddVertex(float x, float y, float z)
    {
        var vertex = new Model.Vertex(
            new Vector3(x, y, z),
            new Vector3(0, 0, 1),
            new Vector3(1, 0, 0));
        vertex.UVs.Add(new Vector2(x / 128f, y / 128f));
        vertex.Weights.Add(new Model.Vertex.Weight(0, 1f));
        vertex.Color = new Vector4(0.25f, 0.5f, 0.75f, 1f);
        mesh.Vertices.Add(vertex);
    }
}

static object Summarize(string filePath)
{
    var castFile = Cast.CastFile.Load(filePath);
    var models = castFile.RootNodes
        .SelectMany(root => root.ChildrenOfType<Cast.Model>())
        .Select(model =>
        {
            var bones = model.Skeleton()?.Bones() ?? [];
            var meshes = model.Meshes();
            return new
            {
                Bones = bones.Select(bone => new
                {
                    Name = bone.Name(),
                    Parent = bone.ParentIndex(),
                    LocalPosition = Vector3ToArray(bone.LocalPosition()),
                    LocalRotation = Vector4ToArray(bone.LocalRotation()),
                    WorldPosition = Vector3ToArray(bone.WorldPosition()),
                    WorldRotation = Vector4ToArray(bone.WorldRotation()),
                    Scale = Vector3ToArray(bone.Scale())
                }),
                Meshes = meshes.Select(mesh => new
                {
                    Vertices = mesh.VertexCount(),
                    Faces = mesh.FaceCount(),
                    MaximumInfluence = mesh.MaximumWeightInfluence(),
                    PositionChecksum = mesh.VertexPositionBuffer()
                        .Aggregate(0d, (sum, point) => sum + point.X + point.Y + point.Z),
                    IndexChecksum = mesh.FaceBuffer().Aggregate(0L, (sum, index) => sum + index),
                    PositionHash = HashVector3(mesh.VertexPositionBuffer()),
                    NormalHash = HashVector3(mesh.VertexNormalBuffer()),
                    ColorHash = HashUInt32(mesh.VertexColorBuffer()),
                    Uv0Hash = HashVector2(mesh.VertexUVLayerBuffer(0)),
                    WeightBoneHash = HashInt32(mesh.VertexWeightBoneBuffer()),
                    WeightValueHash = HashFloat(mesh.VertexWeightValueBuffer()),
                    FaceHash = HashInt32(mesh.FaceBuffer()),
                    Material = mesh.Material()?.Name()
                }),
                Materials = model.Materials().Select(material => material.Name()),
                BlendShapes = model.BlendShapes().Select(shape => new
                {
                    Name = shape.Name(),
                    VertexCount = shape.TargetShapeVertexIndices().Count(),
                    IndexHash = HashInt32(shape.TargetShapeVertexIndices()),
                    TargetPositionHash = HashVector3(shape.TargetShapeVertexPositions()),
                    TargetScale = shape.TargetWeightScale(),
                    BaseMeshHash = shape.BaseShape()?.Hash
                })
            };
        });
    return new
    {
        File = Path.GetFileName(filePath),
        RootCount = castFile.RootNodes.Count,
        Models = models
    };
}

static int Compare(string leftPath, string rightPath)
{
    var left = JsonSerializer.SerializeToNode(Summarize(leftPath))!.AsObject();
    var right = JsonSerializer.SerializeToNode(Summarize(rightPath))!.AsObject();
    left.Remove("File");
    right.Remove("File");
    if (JsonNode.DeepEquals(left, right))
    {
        Console.WriteLine("Semantic summaries match.");
        return 0;
    }

    Console.Error.WriteLine("Semantic summaries differ.");
    Console.Error.WriteLine(JsonSerializer.Serialize(new { Left = left, Right = right },
        new JsonSerializerOptions { WriteIndented = true }));
    return 1;
}

static async Task BenchmarkAsync(
    IModelMergeService service,
    string engine,
    string inputDirectory,
    int partCount,
    int iterations,
    string outputDirectory)
{
    Directory.CreateDirectory(outputDirectory);
    var inputs = Directory.GetFiles(inputDirectory, "part-*.cast")
        .OrderBy(path => path, StringComparer.OrdinalIgnoreCase)
        .Take(partCount)
        .ToArray();
    if (inputs.Length != partCount)
    {
        throw new InvalidOperationException($"Expected {partCount} parts but found {inputs.Length}.");
    }

    var elapsed = new List<double>(iterations);
    for (var iteration = 0; iteration < iterations; iteration++)
    {
        var outputName = $"{engine}-{partCount:D2}-{iteration:D2}.cast";
        var stopwatch = Stopwatch.StartNew();
        await service.MergeAsync(
            new MergeRequest(inputs, outputDirectory, outputName, Overwrite: true));
        stopwatch.Stop();
        elapsed.Add(stopwatch.Elapsed.TotalMilliseconds);
    }

    Console.WriteLine(JsonSerializer.Serialize(new
    {
        Engine = engine,
        Parts = partCount,
        Iterations = iterations,
        Milliseconds = elapsed,
        MedianMilliseconds = Median(elapsed)
    }));
}

static async Task<int> ComparePreviewAsync(
    string workerPath,
    string filePath,
    int triangleLimit,
    int iterations)
{
    var engines = new (string Name, IModelPreviewService Service)[]
    {
        ("csharp", new ModelPreviewService()),
        ("rust-worker", new RustWorkerModelPreviewService(workerPath))
    };
    JsonNode? baseline = null;
    var timings = new Dictionary<string, List<double>>();
    foreach (var (name, service) in engines)
    {
        var elapsed = new List<double>(iterations);
        JsonNode? summary = null;
        for (var iteration = 0; iteration < iterations; iteration++)
        {
            var stopwatch = Stopwatch.StartNew();
            var preview = await service.LoadAsync(filePath, triangleLimit);
            stopwatch.Stop();
            elapsed.Add(stopwatch.Elapsed.TotalMilliseconds);
            summary = JsonSerializer.SerializeToNode(SummarizePreview(preview));
        }

        baseline ??= summary;
        if (!JsonNode.DeepEquals(baseline, summary))
        {
            Console.Error.WriteLine(JsonSerializer.Serialize(
                new { Error = "Preview semantic summaries differ.", Baseline = baseline, Actual = summary },
                new JsonSerializerOptions { WriteIndented = true }));
            return 1;
        }
        timings.Add(name, elapsed);
    }

    Console.WriteLine(JsonSerializer.Serialize(new
    {
        File = Path.GetFileName(filePath),
        TriangleLimit = triangleLimit,
        Iterations = iterations,
        SemanticsMatch = true,
        Engines = timings.ToDictionary(
            pair => pair.Key,
            pair => new { Milliseconds = pair.Value, MedianMilliseconds = Median(pair.Value) })
    }));
    return 0;
}

static object SummarizePreview(ModelPreviewData preview) => new
{
    preview.ModelName,
    preview.SourceMeshCount,
    preview.SourceVertexCount,
    preview.SourceTriangleCount,
    preview.DisplayedTriangleCount,
    preview.IsSimplified,
    preview.Bounds,
    Meshes = preview.Meshes.Select(mesh => new
    {
        PositionHash = HashPreviewPoint3(mesh.Positions),
        NormalHash = HashPreviewPoint3(mesh.Normals),
        IndexHash = HashInt32(mesh.TriangleIndices)
    })
};

static async Task BenchmarkPreviewAsync(
    IModelPreviewService service,
    string engine,
    string filePath,
    int triangleLimit,
    int iterations)
{
    var elapsed = new List<double>(iterations);
    ModelPreviewData? preview = null;
    for (var iteration = 0; iteration < iterations; iteration++)
    {
        var stopwatch = Stopwatch.StartNew();
        preview = await service.LoadAsync(filePath, triangleLimit);
        stopwatch.Stop();
        elapsed.Add(stopwatch.Elapsed.TotalMilliseconds);
    }

    Console.WriteLine(JsonSerializer.Serialize(new
    {
        Engine = engine,
        File = Path.GetFileName(filePath),
        TriangleLimit = triangleLimit,
        Iterations = iterations,
        Milliseconds = elapsed,
        MedianMilliseconds = Median(elapsed),
        Summary = SummarizePreview(preview!)
    }));
}

static double Median(List<double> values)
{
    var sorted = values.Order().ToArray();
    var middle = sorted.Length / 2;
    return sorted.Length % 2 == 0
        ? (sorted[middle - 1] + sorted[middle]) / 2
        : sorted[middle];
}

static float[] Vector3ToArray(Cast.Vector3? vector) =>
    vector is null ? [] : [vector.X, vector.Y, vector.Z];

static float[] Vector4ToArray(Cast.Vector4? vector) =>
    vector is null ? [] : [vector.X, vector.Y, vector.Z, vector.W];

static string HashVector2(IEnumerable<Cast.Vector2> values) => Hash(values, (hash, value) =>
{
    AppendFloat(hash, value.X);
    AppendFloat(hash, value.Y);
});

static string HashVector3(IEnumerable<Cast.Vector3> values) => Hash(values, (hash, value) =>
{
    AppendFloat(hash, value.X);
    AppendFloat(hash, value.Y);
    AppendFloat(hash, value.Z);
});

static string HashPreviewPoint3(IEnumerable<PreviewPoint3> values) => Hash(values, (hash, value) =>
{
    AppendFloat(hash, value.X);
    AppendFloat(hash, value.Y);
    AppendFloat(hash, value.Z);
});

static string HashFloat(IEnumerable<float> values) => Hash(values, AppendFloat);

static string HashInt32(IEnumerable<int> values) => Hash(values, (hash, value) =>
{
    Span<byte> bytes = stackalloc byte[sizeof(int)];
    BinaryPrimitives.WriteInt32LittleEndian(bytes, value);
    hash.AppendData(bytes);
});

static string HashUInt32(IEnumerable<uint> values) => Hash(values, (hash, value) =>
{
    Span<byte> bytes = stackalloc byte[sizeof(uint)];
    BinaryPrimitives.WriteUInt32LittleEndian(bytes, value);
    hash.AppendData(bytes);
});

static string Hash<T>(IEnumerable<T> values, Action<IncrementalHash, T> append)
{
    using var hash = IncrementalHash.CreateHash(HashAlgorithmName.SHA256);
    foreach (var value in values)
    {
        append(hash, value);
    }

    return Convert.ToHexString(hash.GetHashAndReset());
}

static void AppendFloat(IncrementalHash hash, float value)
{
    Span<byte> bytes = stackalloc byte[sizeof(float)];
    BinaryPrimitives.WriteSingleLittleEndian(bytes, value);
    hash.AppendData(bytes);
}

static Quaternion IdentityRotation() => new(0, 0, 0, 1);
