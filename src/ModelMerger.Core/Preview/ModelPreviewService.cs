using ModelMerger.Core.Merging;
using PhilLibX;

namespace ModelMerger.Core.Preview;

public sealed class ModelPreviewService : IModelPreviewService
{
    public const int DefaultTriangleLimit = 75_000;

    public async Task<ModelPreviewData> LoadAsync(
        string filePath,
        int triangleLimit = DefaultTriangleLimit,
        CancellationToken cancellationToken = default)
    {
        if (triangleLimit < 1)
        {
            throw new ArgumentOutOfRangeException(nameof(triangleLimit));
        }

        var normalizedPath = NormalizePath(filePath);
        if (!File.Exists(normalizedPath))
        {
            throw new ModelPreviewException(ModelPreviewErrorCode.MissingFile, normalizedPath);
        }

        if (!string.Equals(Path.GetExtension(normalizedPath), ".cast", StringComparison.OrdinalIgnoreCase))
        {
            throw new ModelPreviewException(ModelPreviewErrorCode.UnsupportedFormat, normalizedPath);
        }

        return await Task.Run(
            () => Load(normalizedPath, triangleLimit, cancellationToken),
            cancellationToken).ConfigureAwait(false);
    }

    private static string NormalizePath(string? filePath)
    {
        try
        {
            if (!string.IsNullOrWhiteSpace(filePath))
            {
                return Path.GetFullPath(filePath);
            }
        }
        catch (Exception exception) when (
            exception is ArgumentException or NotSupportedException or PathTooLongException)
        {
            throw new ModelPreviewException(ModelPreviewErrorCode.InvalidPath, filePath, exception);
        }

        throw new ModelPreviewException(ModelPreviewErrorCode.InvalidPath, filePath);
    }

    private static ModelPreviewData Load(
        string filePath,
        int triangleLimit,
        CancellationToken cancellationToken)
    {
        Model model;
        try
        {
            model = new CastModelLoader().Load(filePath, cancellationToken);
        }
        catch (OperationCanceledException)
        {
            throw;
        }
        catch (Exception exception)
        {
            throw new ModelPreviewException(ModelPreviewErrorCode.UnreadableModel, filePath, exception);
        }

        cancellationToken.ThrowIfCancellationRequested();
        var sourceTriangleCounts = model.Meshes.Select(CountTriangles).ToArray();
        var sourceTriangleCount = sourceTriangleCounts.Sum();
        var validTriangleCounts = model.Meshes
            .Select(mesh => CountValidTriangles(mesh, cancellationToken))
            .ToArray();
        var validTriangleCount = validTriangleCounts.Sum();
        if (validTriangleCount == 0)
        {
            throw new ModelPreviewException(ModelPreviewErrorCode.NoGeometry, filePath);
        }

        var quotas = AllocateTriangleQuotas(validTriangleCounts, Math.Min(triangleLimit, validTriangleCount));
        var meshes = new List<PreviewMeshData>();
        for (var index = 0; index < model.Meshes.Count; index++)
        {
            cancellationToken.ThrowIfCancellationRequested();
            if (quotas[index] == 0)
            {
                continue;
            }

            var previewMesh = BuildPreviewMesh(
                model.Meshes[index],
                validTriangleCounts[index],
                quotas[index],
                cancellationToken);
            if (previewMesh.TriangleIndices.Count > 0)
            {
                meshes.Add(previewMesh);
            }
        }

        if (meshes.Count == 0)
        {
            throw new ModelPreviewException(ModelPreviewErrorCode.NoGeometry, filePath);
        }

        var displayedTriangleCount = meshes.Sum(mesh => mesh.TriangleIndices.Count / 3);
        return new ModelPreviewData(
            filePath,
            model.Name,
            model.Meshes.Count,
            model.Meshes.Sum(mesh => mesh.Vertices.Count),
            sourceTriangleCount,
            displayedTriangleCount,
            displayedTriangleCount < sourceTriangleCount,
            CalculateBounds(meshes),
            meshes);
    }

    private static int CountTriangles(Model.Mesh mesh) =>
        mesh.Faces.Sum(face => Math.Max(0, face.Indices.Length - 2));

    private static int CountValidTriangles(Model.Mesh mesh, CancellationToken cancellationToken)
    {
        var count = 0;
        foreach (var face in mesh.Faces)
        {
            cancellationToken.ThrowIfCancellationRequested();
            for (var index = 1; index < face.Indices.Length - 1; index++)
            {
                if (IsValidTriangle(mesh, face.Indices[0], face.Indices[index], face.Indices[index + 1]))
                {
                    count++;
                }
            }
        }

        return count;
    }

    internal static int[] AllocateTriangleQuotas(IReadOnlyList<int> triangleCounts, int budget)
    {
        var quotas = new int[triangleCounts.Count];
        var total = triangleCounts.Sum();
        if (budget >= total)
        {
            for (var index = 0; index < triangleCounts.Count; index++)
            {
                quotas[index] = triangleCounts[index];
            }

            return quotas;
        }

        var rankedRemainders = new List<(int Index, double Remainder)>();
        var allocated = 0;
        for (var index = 0; index < triangleCounts.Count; index++)
        {
            var exact = (double)budget * triangleCounts[index] / total;
            quotas[index] = (int)Math.Floor(exact);
            allocated += quotas[index];
            rankedRemainders.Add((index, exact - quotas[index]));
        }

        foreach (var item in rankedRemainders
                     .OrderByDescending(item => item.Remainder)
                     .ThenBy(item => item.Index)
                     .Take(budget - allocated))
        {
            quotas[item.Index]++;
        }

        return quotas;
    }

    private static PreviewMeshData BuildPreviewMesh(
        Model.Mesh mesh,
        int sourceTriangleCount,
        int quota,
        CancellationToken cancellationToken)
    {
        var selectedOrdinals = SelectTriangleOrdinals(sourceTriangleCount, quota);
        var positions = new List<PreviewPoint3>();
        var normals = new List<PreviewPoint3>();
        var indices = new List<int>(quota * 3);
        var vertexMap = new Dictionary<int, int>();
        var validTriangleOrdinal = 0;

        foreach (var face in mesh.Faces)
        {
            cancellationToken.ThrowIfCancellationRequested();
            for (var index = 1; index < face.Indices.Length - 1; index++)
            {
                var first = face.Indices[0];
                var second = face.Indices[index];
                var third = face.Indices[index + 1];
                if (!IsValidTriangle(mesh, first, second, third))
                {
                    continue;
                }

                if (selectedOrdinals.Contains(validTriangleOrdinal))
                {
                    AddTriangle(first, second, third);
                }

                validTriangleOrdinal++;
            }
        }

        return new PreviewMeshData(positions, normals, indices);

        void AddTriangle(int first, int second, int third)
        {
            indices.Add(GetOrAddVertex(first));
            indices.Add(GetOrAddVertex(second));
            indices.Add(GetOrAddVertex(third));
        }

        int GetOrAddVertex(int sourceIndex)
        {
            if (vertexMap.TryGetValue(sourceIndex, out var existing))
            {
                return existing;
            }

            var vertex = mesh.Vertices[sourceIndex];
            var previewIndex = positions.Count;
            positions.Add(new PreviewPoint3(vertex.Position.X, vertex.Position.Y, vertex.Position.Z));
            var normal = vertex.Normal;
            normals.Add(float.IsFinite(normal.X) && float.IsFinite(normal.Y) && float.IsFinite(normal.Z)
                ? new PreviewPoint3(normal.X, normal.Y, normal.Z)
                : new PreviewPoint3(0, 0, 0));
            vertexMap.Add(sourceIndex, previewIndex);
            return previewIndex;
        }
    }

    internal static HashSet<int> SelectTriangleOrdinals(int sourceTriangleCount, int quota) =>
        Enumerable.Range(0, quota)
            .Select(index => (int)Math.Floor((double)index * sourceTriangleCount / quota))
            .ToHashSet();

    private static bool IsValidTriangle(Model.Mesh mesh, int first, int second, int third) =>
        IsValidVertex(mesh, first) && IsValidVertex(mesh, second) && IsValidVertex(mesh, third);

    private static bool IsValidVertex(Model.Mesh mesh, int index)
    {
        if (index < 0 || index >= mesh.Vertices.Count)
        {
            return false;
        }

        var position = mesh.Vertices[index].Position;
        return float.IsFinite(position.X) && float.IsFinite(position.Y) && float.IsFinite(position.Z);
    }

    private static PreviewBounds CalculateBounds(IReadOnlyList<PreviewMeshData> meshes)
    {
        var minimumX = float.PositiveInfinity;
        var minimumY = float.PositiveInfinity;
        var minimumZ = float.PositiveInfinity;
        var maximumX = float.NegativeInfinity;
        var maximumY = float.NegativeInfinity;
        var maximumZ = float.NegativeInfinity;
        foreach (var mesh in meshes)
        {
            foreach (var point in mesh.Positions)
            {
                minimumX = Math.Min(minimumX, point.X);
                minimumY = Math.Min(minimumY, point.Y);
                minimumZ = Math.Min(minimumZ, point.Z);
                maximumX = Math.Max(maximumX, point.X);
                maximumY = Math.Max(maximumY, point.Y);
                maximumZ = Math.Max(maximumZ, point.Z);
            }
        }

        return new PreviewBounds(
            new PreviewPoint3(minimumX, minimumY, minimumZ),
            new PreviewPoint3(maximumX, maximumY, maximumZ));
    }
}
