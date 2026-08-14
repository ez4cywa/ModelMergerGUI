using PhilLibX;

namespace ModelMerger.Core.Merging;

internal interface IModelPartLoader
{
    string Extension { get; }

    string FormatName { get; }

    Model Load(string filePath, CancellationToken cancellationToken);
}
