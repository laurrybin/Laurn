// Copyright 2026 Darwin Clay O. and Lawrence Obina
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

using System.IO;
using UnrealBuildTool;

public class Laurn : ModuleRules
{
    public Laurn( ReadOnlyTargetRules Target ) : base(Target)
    {
        PCHUsage = ModuleRules.PCHUsageMode.UseExplicitOrSharedPCHs;

        PublicDependencyModuleNames.Add("Core");

        PrivateDependencyModuleNames.AddRange(
            new string[]
            {
                "CoreUObject",
                "Engine"
            }
        );

        string RustLibDir = Path.GetFullPath(
            Path.Combine(ModuleDirectory, "../../../../target/debug")
        );

        string RustLibPath;

        if (Target.Platform == UnrealTargetPlatform.Win64)
        {
            RustLibPath = Path.Combine(RustLibDir, "laurn_c.lib");
            PublicAdditionalLibraries.Add("advapi32.lib");
            PublicAdditionalLibraries.Add("ws2_32.lib");
            PublicAdditionalLibraries.Add("userenv.lib");
            PublicAdditionalLibraries.Add("bcrypt.lib");
        }
        else if (Target.Platform == UnrealTargetPlatform.Mac)
        {
            RustLibPath = Path.Combine(RustLibDir, "liblaurn_c.a");
            PublicSystemLibraries.Add("iconv");
            PublicFrameworks.Add("System");
            PublicFrameworks.Add("CoreFoundation");
        }
        else if (Target.Platform == UnrealTargetPlatform.Linux)
        {
            RustLibPath = Path.Combine(RustLibDir, "liblaurn_c.a");
        }
        else
        {
            throw new BuildException(
                "LAURN does not currently configure a Rust FFI library for platform {0}.",
                Target.Platform
            );
        }

        if (!File.Exists(RustLibPath))
        {
            throw new BuildException(
                "LAURN Rust FFI library not found at {0}. Run cargo build -p laurn-c from the repository root before building the Unreal plugin.",
                RustLibPath
            );
        }

        PublicAdditionalLibraries.Add(RustLibPath);
    }
}
