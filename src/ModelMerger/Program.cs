// ------------------------------------------------------------------------
// Model Merger - Simple Tool to Merge Models
// Copyright(c) 2018 Philip/Scobalula
// 
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
// 
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
// 
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
// ------------------------------------------------------------------------
// File: Program.cs
// Author: Philip/Scobalula
// Description: Main App Logic
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using SELib;
using PhilLibX;
using PhilLibX.Mathematics;
using System.Reflection;
using System.Diagnostics;

namespace ModelMerger
{
    class Program
    {
        #region Model Loaders
        private static Model LoadSEModel(string filePath)
        {
            var model = new Model(Path.GetFileNameWithoutExtension(filePath));
            var input = SEModel.Read(filePath);

            Printer.WriteLine("LOADER", string.Format("Loading {0}", model.Name));

            foreach (var shape in input.Shapes)
            {
                model.Shapes.Add(shape);
            }

            foreach (var bone in input.Bones)
            { 
                model.Bones.Add(new Model.Bone(
                    bone.BoneName,
                    bone.BoneParent,
                    new Vector3(
                        (float)bone.LocalPosition.X,
                        (float)bone.LocalPosition.Y,
                        (float)bone.LocalPosition.Z),
                    new Quaternion(
                        (float)bone.LocalRotation.X,
                        (float)bone.LocalRotation.Y,
                        (float)bone.LocalRotation.Z,
                        (float)bone.LocalRotation.W),
                    new Vector3(
                        (float)bone.GlobalPosition.X,
                        (float)bone.GlobalPosition.Y,
                        (float)bone.GlobalPosition.Z),
                    new Quaternion(
                        (float)bone.GlobalRotation.X,
                        (float)bone.GlobalRotation.Y,
                        (float)bone.GlobalRotation.Z,
                        (float)bone.GlobalRotation.W)));
            }

            foreach (var semesh in input.Meshes)
            {
                var mesh = new Model.Mesh((int)semesh.VertexCount, (int)semesh.FaceCount);

                foreach (var mtl in semesh.MaterialReferenceIndicies)
                    mesh.MaterialIndices.Add(mtl);

                foreach (var severtex in semesh.Verticies)
                {
                    var vertex = new Model.Vertex(
                        new Vector3((float)severtex.Position.X, (float)severtex.Position.Y, (float)severtex.Position.Z),
                        new Vector3((float)severtex.VertexNormal.X, (float)severtex.VertexNormal.Y, (float)severtex.VertexNormal.Z));

                    foreach (var uv in severtex.UVSets)
                        vertex.UVs.Add(new Vector2((float)uv.X, (float)uv.Y));

                    foreach (var weight in severtex.Weights)
                        vertex.Weights.Add(new Model.Vertex.Weight((int)weight.BoneIndex, weight.BoneWeight));

                    foreach (var shape in severtex.Shapes)
                        vertex.Shapes.Add(new Model.Vertex.Shape(
                            (int)shape.ShapeIndex,
                            new Vector3(
                                (float)shape.Delta.X,
                                (float)shape.Delta.Y,
                                (float)shape.Delta.Z)));

                    vertex.Color = new Vector4(
                        severtex.VertexColor.R / 255.0f,
                        severtex.VertexColor.G / 255.0f,
                        severtex.VertexColor.B / 255.0f,
                        severtex.VertexColor.A / 255.0f);

                    mesh.Vertices.Add(vertex);
                }

                foreach (var face in semesh.Faces)
                {
                    mesh.Faces.Add(new Model.Face((int)face.FaceIndex1, (int)face.FaceIndex2, (int)face.FaceIndex3));
                }

                model.Meshes.Add(mesh);
            }

            foreach (var material in input.Materials)
            {
                model.Materials.Add(new Model.Material(material.Name));
            }

            Printer.WriteLine("LOADER", string.Format("Loaded {0}", model.Name));

            return model;
        }

        private static Model LoadCastModel(string filePath)
        {
            var model = new Model(Path.GetFileNameWithoutExtension(filePath));
            var CastFile = Cast.CastFile.Load(filePath);
            var Model = CastFile.RootNodes.Count > 0 ? CastFile.RootNodes[0].ChildrenOfType<Cast.Model>().FirstOrDefault() : null;

            if (Model == null)
                throw new Exception(string.Format("{0} does not contain a model", Path.GetFileName(filePath)));

            var Skeleton = Model.Skeleton();
            var Mats = Model.ChildrenOfType<Cast.Material>();

            Printer.WriteLine("LOADER", string.Format("Loading {0}", Path.GetFileName(filePath)));

            foreach (var blend in Model.BlendShapes())
            {
                model.Shapes.Add(blend.Name());
            }

            var SkeletonBones = Skeleton != null ? Skeleton.Bones() : new List<Cast.Bone>();

            foreach (var bone in SkeletonBones)
            {
                var boneLocPos = bone.LocalPosition();
                var boneLocRot = bone.LocalRotation();
                var boneGlobalRot = bone.WorldRotation();
                var boneGlobalPos = bone.WorldPosition();
                model.Bones.Add(new Model.Bone(
                    bone.Name(),
                    bone.ParentIndex(),
                    new Vector3(boneLocPos.X, boneLocPos.Y, boneLocPos.Z),
                    new Quaternion(boneLocRot.X, boneLocRot.Y, boneLocRot.Z, boneLocRot.W),
                    new Vector3(boneGlobalPos.X, boneGlobalPos.Y, boneGlobalPos.Z),
                    new Quaternion(boneGlobalRot.X, boneGlobalRot.Y, boneGlobalRot.Z, boneGlobalRot.W)));
            }

            foreach(var CastMesh in Model.Meshes())
            {
                var mesh = new Model.Mesh(CastMesh.VertexCount(), CastMesh.FaceCount());

                var VertexBuffer = CastMesh.VertexPositionBuffer().ToArray();
                var NormalBuffer = CastMesh.VertexNormalBuffer().ToArray();
                var WeightBoneBuffer = CastMesh.VertexWeightBoneBuffer().ToArray();
                var WeightValueBuffer = CastMesh.VertexWeightValueBuffer().ToArray();
                var FaceBuffer = CastMesh.FaceBuffer().ToArray();
                var UVBuffer = CastMesh.UVLayerCount() > 0 ? CastMesh.VertexUVLayerBuffer(0).ToArray() : new Cast.Vector2[0];
                var ColorBuffer = CastMesh.VertexColorBuffer().ToArray();

                var MaterialIndex = Mats.IndexOf(CastMesh.Material());

                if (MaterialIndex > -1)
                    mesh.MaterialIndices.Add(MaterialIndex);

                var BlendsForMesh = Model.BlendShapes().FindAll(x => x.BaseShape() == CastMesh);

                for (int i = 0; i < CastMesh.VertexCount(); i++)
                {
                    var vert = new Model.Vertex(
                        new Vector3(VertexBuffer[i].X, VertexBuffer[i].Y, VertexBuffer[i].Z),
                        new Vector3(NormalBuffer[i].X, NormalBuffer[i].Y, NormalBuffer[i].Z));

                    vert.UVs.Add(i < UVBuffer.Length ? new Vector2(UVBuffer[i].X, UVBuffer[i].Y) : new Vector2(0, 0));
                    int weightStartIndex = i * CastMesh.MaximumWeightInfluence();
                    for (int j = 0; j < CastMesh.MaximumWeightInfluence(); j++)
                    {
                        var weight = new Model.Vertex.Weight(WeightBoneBuffer[weightStartIndex + j], WeightValueBuffer[weightStartIndex + j]);
                        vert.Weights.Add(weight);
                    }

                    if (i < ColorBuffer.Length)
                    {
                        vert.Color = new Vector4(
                            (ColorBuffer[i] & 0xFF) / 255.0f,
                            ((ColorBuffer[i] >> 8) & 0xFF) / 255.0f,
                            ((ColorBuffer[i] >> 16) & 0xFF) / 255.0f,
                            ((ColorBuffer[i] >> 24) & 0xFF) / 255.0f);
                    }
                    else
                    {
                        vert.Color = new Vector4(1, 1, 1, 1);
                    }

                    mesh.Vertices.Add(vert);
                }

                foreach (var BlendForMesh in BlendsForMesh)
                {
                    var BlendPositions = BlendForMesh.TargetShapeVertexPositions().ToArray();
                    var BlendIndices = BlendForMesh.TargetShapeVertexIndices().ToArray();
                    var index = model.Shapes.IndexOf(BlendForMesh.Name());

                    for (int i = 0; i < BlendIndices.Length; i++)
                    {
                        // Cast stores absolute target positions, internally we keep
                        // deltas (SEModel convention) so merging can rotate them and
                        // each writer can produce its own convention from one source
                        var currentVertex = mesh.Vertices[BlendIndices[i]];
                        currentVertex.Shapes.Add(new Model.Vertex.Shape(index, new Vector3(
                            BlendPositions[i].X - currentVertex.Position.X,
                            BlendPositions[i].Y - currentVertex.Position.Y,
                            BlendPositions[i].Z - currentVertex.Position.Z)));
                    }
                }

                for (var i = 0; i < FaceBuffer.Count(); i+=3)
                {
                    mesh.Faces.Add(new Model.Face(FaceBuffer[i], FaceBuffer[i + 1], FaceBuffer[i + 2]));
                }

                model.Meshes.Add(mesh);
            }

            foreach (var material in Model.Materials())
            {
                var mat = new Model.Material(material.Name());
                model.Materials.Add(mat);
            }

            Printer.WriteLine("LOADER", string.Format("Loaded {0}", model.Name));

            return model;
        }

        static Model LoadModel(string filePath)
        {
            switch(Path.GetExtension(filePath).ToLower())
            {
                case ".semodel":
                    return LoadSEModel(filePath);
                case ".cast":
                        return LoadCastModel(filePath);
                default:
                    return null;
            }
        }

        static List<Model> LoadModels(string[] args)
        {
            var fileNames = args.ToList();
            var models = new List<Model>(args.Length);

            fileNames.Sort();

            foreach(var fileName in fileNames)
            {
                var model = LoadModel(fileName);

                if (model != null)
                    models.Add(model);
                else
                    Printer.WriteLine("ERROR", string.Format("Invalid file: {0}", Path.GetFileName(fileName)), ConsoleColor.Red);
            }

            return models;
        }
        #endregion

        static bool CanBeConnected(Model input, List<Model> models)
        {
            var root = input.Bones[0].Name;

            foreach(var model in models)
            {
                if(model != input)
                {
                    if (model.HasBone(root))
                    {
                        return true;
                    }
                }
            }

            return false;
        }

        static void MergeModel(Model rootModel, Model model, Logger logger)
        {
            Printer.WriteLine("MERGER", string.Format("Merging {0}", model.Name));
            logger.Write(string.Format("Merging {0}", model.Name), Logger.MessageType.INFO);

            // Add missing bones in two passes so parent resolution never
            // depends on the order the source model stored its bones in:
            // add everything first, resolve parents by name after
            var boneLookup = new Dictionary<string, int>();

            for (int i = 0; i < rootModel.Bones.Count; i++)
                boneLookup.TryAdd(rootModel.Bones[i].Name, i);

            var addedBones = new List<(Model.Bone Source, Model.Bone New)>();

            foreach (var bone in model.Bones)
            {
                if (!boneLookup.ContainsKey(bone.Name))
                {
                    var nBone = new Model.Bone(bone.Name, -1, bone.LocalPosition, bone.LocalRotation);
                    rootModel.Bones.Add(nBone);
                    boneLookup[bone.Name] = rootModel.Bones.Count - 1;
                    addedBones.Add((bone, nBone));
                }
            }

            foreach (var (source, nBone) in addedBones)
            {
                if (source.ParentIndex > -1)
                    nBone.ParentIndex = boneLookup.TryGetValue(model.Bones[source.ParentIndex].Name, out var parentIndex) ? parentIndex : -1;
            }

            foreach (var shape in model.Shapes)
            {
                if (!rootModel.Shapes.Contains(shape))
                {
                    rootModel.Shapes.Add(shape);
                }
            }

            // Compute global positions (we need them for offsetting)
            rootModel.GenerateGlobalBoneData();
            model.GenerateGlobalBoneData();

            // Get root and the new root, to compute offsets
            // TODO: compute this for each bone and utilize weights
            // but as an option, as it may cause severe deformations
            // if bones have moved
            var translation = new Vector3(0, 0, 0);
            var rotation = new Quaternion(0, 0, 0, 1).ToMatrix();

            if (model.Bones.Count > 0)
            {
                var root = model.Bones[0];
                var nRoot = rootModel.Bones[boneLookup[root.Name]];

                translation = nRoot.GlobalPosition - root.GlobalPosition;
                rotation = (nRoot.GlobalRotation * root.GlobalRotation.Inverse()).ToMatrix();
            }

            foreach (var material in model.Materials)
            {
                if (rootModel.Materials.Find(x => x.Name == material.Name) == null)
                {
                    rootModel.Materials.Add(material);
                }
            }

            // Resolve every index remap once, per-vertex lookups by name
            // made big merges crawl
            var boneRemap = model.Bones.Select(x => boneLookup[x.Name]).ToArray();
            var shapeRemap = model.Shapes.Select(x => rootModel.Shapes.IndexOf(x)).ToArray();
            var materialRemap = model.Materials.Select(m => rootModel.Materials.FindIndex(x => x.Name == m.Name)).ToArray();

            foreach (var mesh in model.Meshes)
            {
                var nMesh = new Model.Mesh(mesh.Vertices.Count, 0)
                {
                    Faces = new List<Model.Face>(mesh.Faces)
                };

                foreach (var material in mesh.MaterialIndices)
                {
                    nMesh.MaterialIndices.Add(materialRemap[material]);
                }

                foreach (var vertex in mesh.Vertices)
                {
                    var nVertex = new Model.Vertex(vertex.Position, vertex.Normal, vertex.Tangent)
                    {
                        Color = vertex.Color,
                        Weights = new List<Model.Vertex.Weight>(vertex.Weights.Count),
                        UVs = new List<Vector2>(vertex.UVs)
                    };

                    foreach (var weight in vertex.Weights)
                    {
                        nVertex.Weights.Add(new Model.Vertex.Weight(boneRemap[weight.BoneIndex], weight.Influence));
                    }

                    foreach (var shape in vertex.Shapes)
                    {
                        nVertex.Shapes.Add(new Model.Vertex.Shape(
                            shapeRemap[shape.ShapeIndex],
                            rotation.TransformVector(shape.Delta)));
                    }

                    // Now move it to the new position
                    nVertex.Position = rotation.TransformVector(vertex.Position);
                    nVertex.Normal = rotation.TransformVector(vertex.Normal);
                    nVertex.Position += translation;

                    nMesh.Vertices.Add(nVertex);
                }

                rootModel.Meshes.Add(nMesh);
            }

            Printer.WriteLine("MERGER", string.Format("Merged {0}", model.Name));
            logger.Write(string.Format("Merged {0}", model.Name), Logger.MessageType.INFO);
        }

        static Model GetRootModel(List<Model> models)
        {
            foreach(var a in models)
            {
                if(a.Bones.Count > 0)
                {
                    if(!CanBeConnected(a, models))
                    {
                        return a;
                    }
                }
            }

            return models[0];
        }

        static void Main(string[] args)
        {
            var logger = new Logger("ModelMerger", Path.Combine(Path.GetDirectoryName(AppContext.BaseDirectory), "ModelMerger.log"));
            Printer.WriteLine("INIT", "---------------------------");
            Printer.WriteLine("INIT", "ModelMerger by Scobalula");
            Printer.WriteLine("INIT", "Cast Support by echo000");
            Printer.WriteLine("INIT", "Merges SEModels/Cast Models into 1");
            Printer.WriteLine("INIT", string.Format("Version {0}", Assembly.GetExecutingAssembly().GetName().Version));
            Printer.WriteLine("INIT", "---------------------------");

            logger.Write("---------------------------", Logger.MessageType.INFO);
            logger.Write("ModelMerger by Scobalula", Logger.MessageType.INFO);
            logger.Write("Cast Support by echo000", Logger.MessageType.INFO);
            logger.Write("Merges SEModels/Cast Models into 1", Logger.MessageType.INFO);
            logger.Write(string.Format("Version {0}", Assembly.GetExecutingAssembly().GetName().Version), Logger.MessageType.INFO);
            logger.Write("---------------------------", Logger.MessageType.INFO);

            try
            {
                var models = LoadModels(args);

                if (models.Count == 0)
                {
                    Printer.WriteLine("USAGE", "Simply drag and drop supported model files onto the exe");
                }
                else
                {
                    var rootModel = GetRootModel(models);

                    if (rootModel == null)
                        throw new Exception("Failed to obtain root model");

                    Printer.WriteLine("MERGER", string.Format("Using {0} as root model", rootModel.Name));
                    logger.Write(string.Format("Using {0} as root model", rootModel.Name), Logger.MessageType.INFO);

                    var merged = new List<Model>(models.Count)
                    {
                        rootModel
                    };

                    // Save next to the dropped files rather than next to the exe,
                    // the exe may live somewhere the user can't write to
                    var firstInput = args.FirstOrDefault(File.Exists);
                    var outputFolder = Path.Combine(
                        firstInput != null ? Path.GetDirectoryName(Path.GetFullPath(firstInput)) : Path.GetDirectoryName(AppContext.BaseDirectory),
                        "Merged Models");

                    // Keep looping until we've resolved all models
                    // We do this because some models connect to other
                    // so we need to wait until we've processed that model
                    while (merged.Count < models.Count)
                    {
                        var progressed = false;

                        foreach (var model in models)
                        {
                            // Check if we've processed, also considers root as it's added to merged
                            if (merged.Contains(model))
                                continue;
                            // If we have a model that doesn't exist, and can be connected, we must wait
                            // for it's parent model to be connected
                            if (model.Bones.Count > 0 && !rootModel.HasBone(model.Bones[0].Name) && CanBeConnected(model, models))
                                continue;
                            // Add to the group
                            merged.Add(model);
                            MergeModel(rootModel, model, logger);
                            progressed = true;
                        }

                        // A full pass with no progress means the remaining models only
                        // connect to each other, never to the root's hierarchy. Attach
                        // the first one as-is so the rest can connect through it,
                        // instead of looping forever
                        if (!progressed)
                        {
                            var stuck = models.First(x => !merged.Contains(x));
                            Printer.WriteLine("MERGER", string.Format("{0} shares no bones with {1}, attaching without repositioning", stuck.Name, rootModel.Name), ConsoleColor.DarkYellow);
                            logger.Write(string.Format("{0} shares no bones with {1}, attaching without repositioning", stuck.Name, rootModel.Name), Logger.MessageType.WARNING);
                            merged.Add(stuck);
                            MergeModel(rootModel, stuck, logger);
                        }
                    }

                    Printer.WriteLine("MERGER", string.Format("Saving {0}", rootModel.Name));
                    logger.Write(string.Format("Saving {0}", rootModel.Name), Logger.MessageType.INFO);
                    Directory.CreateDirectory(outputFolder);
                    rootModel.Save(Path.Combine(outputFolder, rootModel.Name + ".cast"));
                    Printer.WriteLine("MERGER", string.Format("Saved {0}", rootModel.Name));
                    logger.Write(string.Format("Saved {0}", rootModel.Name), Logger.MessageType.INFO);
                }
            }
            catch (Exception e)
            {
                logger.Write("An unhandled exception has occured:", Logger.MessageType.ERROR);
                Printer.WriteLine("ERROR", "An unhandled exception has occured:", ConsoleColor.DarkRed);
                logger.Write(e, Logger.MessageType.ERROR);
                Console.WriteLine(e);
            }


            Printer.WriteLine("DONE", "Execution complete, press Enter to exit");
            logger.Write("Execution complete, press Enter to exit", Logger.MessageType.INFO);
            logger.Flush();
            Console.ReadLine();
        }
    }
}
