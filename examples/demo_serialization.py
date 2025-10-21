"""
Demo of JSON serialization for HogTrace programs.

This demonstrates the typical workflow for storing and retrieving
probe definitions in a database.
"""

import hogtrace
import json

print("=" * 70)
print("HogTrace JSON Serialization Demo")
print("=" * 70)
print()

# Step 1: User writes HogTrace code in the UI
print("1. User writes probe definition in UI:")
print("-" * 70)

user_code = """
fn:myapp.users.create:entry
/ arg0.role == "admin" /
{
    $req.user_id = arg0.id;
    $req.timestamp = timestamp();
    capture(
        user_id=arg0.id,
        role=arg0.role,
        email=arg0.email
    );
}

fn:myapp.users.create:exit
{
    capture(
        user_id=$req.user_id,
        duration=timestamp() - $req.timestamp,
        result=retval
    );
}
"""

print(user_code)

# Step 2: Backend parses and validates
print("2. Backend parses and validates the code:")
print("-" * 70)

try:
    program = hogtrace.parse(user_code)
    print(f"✓ Parsed successfully: {len(program.probes)} probes")
    for i, probe in enumerate(program.probes):
        print(f"  - Probe {i+1}: {probe.spec.full_spec}")
except hogtrace.ParseError as e:
    print(f"✗ Parse error: {e}")
    exit(1)

print()

# Step 3: Backend serializes to JSON for database storage
print("3. Backend serializes to JSON for database storage:")
print("-" * 70)

json_definition = hogtrace.program_to_json(program)
print(json_definition[:500] + "...")  # Show first 500 chars

# Get size
json_size = len(json_definition.encode('utf-8'))
print(f"\nJSON size: {json_size} bytes")
print()

# Step 4: Store in database (simulated)
print("4. Store in database:")
print("-" * 70)

debug_session_id = "uuid-abc-123-def-456"
print(f"db.save_probe_definition(")
print(f"    session_id='{debug_session_id}',")
print(f"    definition=json_definition")
print(f")")
print("✓ Stored in database")
print()

# Step 5: Later, instrumentation library fetches from DB
print("5. Instrumentation library fetches probe definition:")
print("-" * 70)

# Simulated DB fetch
fetched_json = json_definition
print(f"fetched_json = db.fetch_probe_definition('{debug_session_id}')")
print(f"✓ Fetched {len(fetched_json)} bytes from database")
print()

# Step 6: Deserialize back to Program AST
print("6. Deserialize JSON back to Program AST:")
print("-" * 70)

restored_program = hogtrace.program_from_json(fetched_json)
print(f"✓ Deserialized successfully: {len(restored_program.probes)} probes")
for i, probe in enumerate(restored_program.probes):
    print(f"  - Probe {i+1}: {probe.spec.full_spec}")
    if probe.predicate:
        print(f"    Predicate: {probe.predicate}")
    print(f"    Actions: {len(probe.actions)}")
print()

# Step 7: Create executor and use it
print("7. Create executor for probe execution:")
print("-" * 70)

from hogtrace import ProgramExecutor, RequestLocalStore

store = RequestLocalStore()
executor = ProgramExecutor(restored_program, store)

print(f"✓ Created executor with {len(executor.executors)} probe executors")
print(f"✓ Ready to execute probes!")
print()

# Demonstrate that the roundtrip preserves structure
print("8. Verification - Compare original vs restored:")
print("-" * 70)

print(f"Original probes:  {len(program.probes)}")
print(f"Restored probes:  {len(restored_program.probes)}")
print()

for i in range(len(program.probes)):
    orig = program.probes[i]
    rest = restored_program.probes[i]

    print(f"Probe {i+1}:")
    print(f"  Spec matches:      {orig.spec.full_spec == rest.spec.full_spec}")
    print(f"  Predicate matches: {(orig.predicate is None) == (rest.predicate is None)}")
    print(f"  Action count:      {len(orig.actions)} == {len(rest.actions)}")

print()

# Show compact vs pretty JSON
print("9. JSON formatting options:")
print("-" * 70)

compact_json = hogtrace.program_to_json(program, indent=None)
pretty_json = hogtrace.program_to_json(program, indent=2)

print(f"Compact JSON size: {len(compact_json.encode('utf-8'))} bytes")
print(f"Pretty JSON size:  {len(pretty_json.encode('utf-8'))} bytes")
print(f"Size difference:   {len(pretty_json) - len(compact_json)} chars")
print()

print("Compact JSON sample:")
print(compact_json[:150] + "...")
print()

print("=" * 70)
print("Demo complete! 🎉")
print("=" * 70)
