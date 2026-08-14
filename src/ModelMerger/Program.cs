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
                Write("WARNING", warning, ConsoleColor.DarkYellow);
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
        public void Report(MergeProgress value) => Write(value.Stage.ToString().ToUpperInvariant(), value.Message);
    }
}
