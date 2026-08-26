using System.Text;

namespace ModelMerger.Core.Preview;

internal static class CastPreviewRouteProbe
{
    private const uint CastMagic = 0x74736163;
    private const uint MeshNode = 0x6873656D;
    private const int NodeHeaderLength = 24;

    public static bool Uses32BitFaceIndices(string filePath)
    {
        using var stream = new FileStream(
            filePath,
            FileMode.Open,
            FileAccess.Read,
            FileShare.Read,
            bufferSize: 64 * 1024,
            FileOptions.SequentialScan);
        using var reader = new BinaryReader(stream, Encoding.UTF8, leaveOpen: true);
        if (reader.ReadUInt32() != CastMagic)
        {
            return false;
        }

        _ = reader.ReadUInt32();
        var rootCount = reader.ReadUInt32();
        _ = reader.ReadUInt32();
        for (var index = 0u; index < rootCount; index++)
        {
            if (NodeUses32BitFaceIndices(reader))
            {
                return true;
            }
        }
        return false;
    }

    private static bool NodeUses32BitFaceIndices(BinaryReader reader)
    {
        var stream = reader.BaseStream;
        var start = stream.Position;
        var identifier = reader.ReadUInt32();
        var nodeLength = reader.ReadUInt32();
        if (nodeLength < NodeHeaderLength || start + nodeLength > stream.Length)
        {
            throw new InvalidDataException("Cast node length is invalid.");
        }

        var nodeEnd = start + nodeLength;
        _ = reader.ReadUInt64();
        var propertyCount = reader.ReadUInt32();
        var childCount = reader.ReadUInt32();
        for (var index = 0u; index < propertyCount; index++)
        {
            var type = reader.ReadBytes(2);
            if (type.Length != 2)
            {
                throw new EndOfStreamException();
            }
            var nameLength = reader.ReadUInt16();
            var valueCount = reader.ReadUInt32();
            var name = Encoding.UTF8.GetString(reader.ReadBytes(nameLength));
            if (identifier == MeshNode && type[0] == (byte)'i' && type[1] == 0 && name == "f")
            {
                return true;
            }
            SkipPropertyValues(reader, type, valueCount);
            if (stream.Position > nodeEnd)
            {
                throw new InvalidDataException("Cast property exceeds its node.");
            }
        }

        for (var index = 0u; index < childCount; index++)
        {
            if (NodeUses32BitFaceIndices(reader))
            {
                return true;
            }
        }
        if (stream.Position != nodeEnd)
        {
            throw new InvalidDataException("Cast node length does not match its contents.");
        }
        return false;
    }

    private static void SkipPropertyValues(BinaryReader reader, byte[] type, uint valueCount)
    {
        if (type[0] == (byte)'s' && type[1] == 0)
        {
            if (valueCount != 1)
            {
                throw new InvalidDataException("Cast string property has an invalid value count.");
            }
            while (reader.ReadByte() != 0)
            {
            }
            return;
        }

        var valueLength = (type[0], type[1]) switch
        {
            ((byte)'b', 0) => 1,
            ((byte)'h', 0) => 2,
            ((byte)'i', 0) or ((byte)'f', 0) => 4,
            ((byte)'l', 0) or ((byte)'d', 0) or ((byte)'2', (byte)'v') => 8,
            ((byte)'3', (byte)'v') => 12,
            ((byte)'4', (byte)'v') => 16,
            _ => throw new InvalidDataException("Cast property type is unsupported.")
        };
        var bytes = checked((long)valueCount * valueLength);
        var target = checked(reader.BaseStream.Position + bytes);
        if (target > reader.BaseStream.Length)
        {
            throw new EndOfStreamException();
        }
        reader.BaseStream.Position = target;
    }
}
