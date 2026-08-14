// ------------------------------------------------------------------------
// Model Merger - Simple Tool to Merge Models
// Copyright(c) 2018 Philip/Scobalula
// Cast support by echo000
// Licensed under the MIT License.
// ------------------------------------------------------------------------
using System.Reflection;
using ModelMerger.Core.Merging;

namespace ModelMerger;

internal static class Program
{
    private static readonly string LogPath = Path.Combine(AppContext.BaseDirectory, "ModelMerger.log");

    private static async Task<int> Main(string[] args)
    {
        Write("INIT", "---------------------------");
        Write("INIT", "ModelMerger by Scobalula");
        Write("INIT", "Cast support by echo000");
        Write("INIT", "Merges SEModel/Cast model parts into one Cast model");
        Write("INIT", $"Version {Assembly.GetExecutingAssembly().GetName().Version}");
        Write("INIT", "---------------------------");

        if (args.Length == 0)
        {
            Write("USAGE", "Drag and drop one or more .cast/.semodel files onto ModelMerger.exe.");
            Finish();
            return 1;
        }

        try
        {
            var firstInput = args.FirstOrDefault(File.Exists);
            var outputBase = firstInput is null
                ? AppContext.BaseDirectory
                : Path.GetDirectoryName(Path.GetFullPath(firstInput))!;
            var outputDirectory = Path.Combine(outputBase, "Merged Models");
            var service = ModelMergeService.CreateForCommandLine();
            var result = await service.MergeAsync(
                new MergeRequest(args, outputDirectory, Overwrite: true),
                new ConsoleMergeProgress());

            foreach (var warning in result.Warnings)
            {
                Write("WARNING", Describe(warning), ConsoleColor.DarkYellow);
            }

            Write("DONE", $"Saved {result.PartCount} part(s) to {result.OutputPath}");
            Finish();
            return 0;
        }
        catch (MergeValidationException exception)
        {
            foreach (var error in exception.Errors)
            {
                Write(
                    "ERROR",
                    error.FilePath is null ? error.Message : $"{error.Message} ({error.FilePath})",
                    ConsoleColor.DarkRed);
            }

            Finish();
            return 2;
        }
        catch (Exception exception)
        {
            Write("ERROR", exception.ToString(), ConsoleColor.DarkRed);
            Finish();
            return 3;
        }
    }

    private static void Finish()
    {
        Write("DONE", "Execution complete.");
        if (!Console.IsInputRedirected)
        {
            Console.WriteLine("Press Enter to exit.");
            Console.ReadLine();
        }
    }

    private static void Write(string category, string message, ConsoleColor color = ConsoleColor.Black)
    {
        var previous = Console.ForegroundColor;
        if (color != ConsoleColor.Black)
        {
            Console.ForegroundColor = color;
        }

        Console.WriteLine($"[{category,-8}] {message}");
        Console.ForegroundColor = previous;

        try
        {
            File.AppendAllText(
                LogPath,
                $"{DateTime.Now:yyyy-MM-dd HH:mm:ss} [{category}] {message}{Environment.NewLine}");
        }
        catch (IOException)
        {
            // Logging must never prevent the merge from running.
        }
        catch (UnauthorizedAccessException)
        {
            // The executable may be installed in a read-only directory.
        }
    }

    private sealed class ConsoleMergeProgress : IProgress<MergeProgress>
    {
        public void Report(MergeProgress value) =>
            Write(value.Stage.ToString().ToUpperInvariant(), Describe(value));
    }

    private static string Describe(MergeProgress progress) => progress.Code switch
    {
        MergeProgressCode.ValidatingRequest => "Validating merge request",
        MergeProgressCode.LoadingFile => $"Loading {progress.Subject}",
        MergeProgressCode.SelectingRootModel => "Selecting root model",
        MergeProgressCode.MergingModel => $"Merging {progress.Subject}",
        MergeProgressCode.SavingFile => $"Saving {progress.Subject}",
        MergeProgressCode.VerifyingCast => "Verifying saved Cast model",
        MergeProgressCode.SavedFile => $"Saved {progress.Subject}",
        _ => "Processing"
    };

    private static string Describe(MergeWarning warning) => warning.Code switch
    {
        MergeWarningCode.NoAttachmentBone =>
            $"{warning.ModelName} shares no attachment bone with {warning.RootModelName}; it was merged without repositioning.",
        MergeWarningCode.UnconnectedHierarchy =>
            $"{warning.ModelName} could not connect to the current hierarchy; it was merged without repositioning.",
        _ => $"Merge warning for {warning.ModelName}."
    };
}
