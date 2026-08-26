namespace ModelMerger.Core.Preview;

internal static class RustPreviewPayloadCodec
{
    public const uint Version = 1;
    internal static ReadOnlySpan<byte> Magic => "MMPV"u8;
    private const int CancellationCheckInterval = 4_096;

    public static IReadOnlyList<PreviewMeshData> Read(
        RustPreviewDescriptor descriptor,
        CancellationToken cancellationToken,
        Action<int>? readProgress = null)
    {
        if (descriptor.PayloadLength < 12 ||
            new FileInfo(descriptor.PayloadPath).Length != descriptor.PayloadLength)
        {
            throw new InvalidDataException("Rust preview payload length does not match its descriptor.");
        }

        using var stream = new FileStream(
            descriptor.PayloadPath,
            FileMode.Open,
            FileAccess.Read,
            FileShare.Read,
            bufferSize: 64 * 1024,
            FileOptions.SequentialScan);
        using var reader = new BinaryReader(stream);
        if (!reader.ReadBytes(Magic.Length).AsSpan().SequenceEqual(Magic) ||
            reader.ReadUInt32() != Version)
        {
            throw new InvalidDataException("Rust preview payload has an unsupported header.");
        }

        var meshCount = ReadCount(reader, descriptor.PayloadLength / 8, "mesh");
        var meshes = new List<PreviewMeshData>(meshCount);
        var displayedTriangles = 0;
        var valuesRead = 0;
        for (var meshIndex = 0; meshIndex < meshCount; meshIndex++)
        {
            CheckCancellation(cancellationToken, readProgress, valuesRead);
            var vertexCount = ReadCount(reader, descriptor.PayloadLength / 24, "vertex");
            var indexCount = ReadCount(reader, descriptor.PayloadLength / 4, "index");
            if (indexCount % 3 != 0)
            {
                throw new InvalidDataException("Rust preview payload index count is not triangular.");
            }

            var positions = new PreviewPoint3[vertexCount];
            var normals = new PreviewPoint3[vertexCount];
            var indices = new int[indexCount];
            for (var index = 0; index < vertexCount; index++)
            {
                CheckCancellationAtInterval(index, cancellationToken, readProgress, valuesRead);
                positions[index] = ReadPoint(reader);
                valuesRead++;
            }
            for (var index = 0; index < vertexCount; index++)
            {
                CheckCancellationAtInterval(index, cancellationToken, readProgress, valuesRead);
                normals[index] = ReadPoint(reader);
                valuesRead++;
            }
            for (var index = 0; index < indexCount; index++)
            {
                CheckCancellationAtInterval(index, cancellationToken, readProgress, valuesRead);
                indices[index] = checked((int)reader.ReadUInt32());
                if ((uint)indices[index] >= vertexCount)
                {
                    throw new InvalidDataException("Rust preview payload references a missing vertex.");
                }
                valuesRead++;
            }
            displayedTriangles = checked(displayedTriangles + indexCount / 3);
            meshes.Add(new PreviewMeshData(positions, normals, indices));
        }

        if (stream.Position != stream.Length ||
            displayedTriangles != descriptor.DisplayedTriangleCount)
        {
            throw new InvalidDataException("Rust preview payload does not match its descriptor.");
        }
        return meshes;
    }

    public static void Write(BinaryWriter writer, IReadOnlyList<PreviewMeshData> meshes)
    {
        writer.Write(Magic);
        writer.Write(Version);
        writer.Write(checked((uint)meshes.Count));
        foreach (var mesh in meshes)
        {
            writer.Write(checked((uint)mesh.Positions.Count));
            writer.Write(checked((uint)mesh.TriangleIndices.Count));
            foreach (var point in mesh.Positions)
            {
                WritePoint(writer, point);
            }
            foreach (var normal in mesh.Normals)
            {
                WritePoint(writer, normal);
            }
            foreach (var index in mesh.TriangleIndices)
            {
                writer.Write(checked((uint)index));
            }
        }
    }

    private static int ReadCount(BinaryReader reader, long maximum, string kind)
    {
        var value = reader.ReadUInt32();
        if (value > int.MaxValue || value > maximum)
        {
            throw new InvalidDataException($"Rust preview payload contains an invalid {kind} count.");
        }
        return (int)value;
    }

    private static PreviewPoint3 ReadPoint(BinaryReader reader) =>
        new(reader.ReadSingle(), reader.ReadSingle(), reader.ReadSingle());

    private static void WritePoint(BinaryWriter writer, PreviewPoint3 point)
    {
        writer.Write(point.X);
        writer.Write(point.Y);
        writer.Write(point.Z);
    }

    private static void CheckCancellationAtInterval(
        int index,
        CancellationToken cancellationToken,
        Action<int>? readProgress,
        int valuesRead)
    {
        if (index % CancellationCheckInterval == 0)
        {
            CheckCancellation(cancellationToken, readProgress, valuesRead);
        }
    }

    private static void CheckCancellation(
        CancellationToken cancellationToken,
        Action<int>? readProgress,
        int valuesRead)
    {
        readProgress?.Invoke(valuesRead);
        cancellationToken.ThrowIfCancellationRequested();
    }
}
