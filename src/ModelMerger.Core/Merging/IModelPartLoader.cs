using PhilLibX;

namespace ModelMerger.Core.Merging;

internal interface IModelPartLoader
{
    string Extension { get; }

    Model Load(string filePath, CancellationToken cancellationToken);
}
